// NOTE: Set "windows" subsystem only for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate image;
extern crate clap;
#[macro_use]
extern crate log;
extern crate simple_logging;
extern crate winreg;
extern crate winapi;
extern crate user32;

pub mod app_error;

use std::time::{SystemTime, UNIX_EPOCH};
use std::io::{Read};
use std::env::{current_dir};
use std::fs::{DirBuilder};
use std::process::{exit};
use std::path::{Path, PathBuf};
use std::io;

use chrono::prelude::*;
use chrono::offset::{Utc};

use image::{GenericImage, ImageBuffer, ImageFormat, load_from_memory_with_format};

use clap::{App, Arg};

use app_error::{AppErr};

#[derive(Serialize, Deserialize, Debug)]
struct LatestInfo {
    date: String,
    file: String
}

#[cfg(debug_assertions)]
fn initialize_logger () -> io::Result<()> {
    simple_logging::log_to_stderr(log::LogLevelFilter::Info)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

#[cfg(not(debug_assertions))]
fn initialize_logger () -> io::Result<()> {
    simple_logging::log_to_file("himawari-desktop-updater.log", log::LogLevelFilter::Info)
}

fn main () {
    
    let args = 
        App::new("himawari-desktop-updater")
            .version("0.1")
            .about("Downloads the latest photo from the Himawari-8 geo-synchronous satellite, and sets it as your desktop background (TODO)")
            .author("Benjamin Fox")

            .arg(Arg::with_name("store-latest-only")
                .long("store-latest-only")
                .help("If set, writes the output to a single file named 'latest.png'"))

            .arg(Arg::with_name("force")
                .long("force")
                .help("If set, allow the output file to be overwritten"))

            .arg(Arg::with_name("output-dir")
                .long("output-dir")
                .help("Set the output directory")
                .value_name("OUTPUT_DIR"))

            .get_matches();

    // If set, write only to "latest.png"
    let store_latest_only = args.is_present("store-latest-only");

    // If set, overwrite output image
    let force = args.is_present("force");

    // Directory to write images out to
    let output_dir = args.value_of("output-dir")
                        .map(|s| Path::new(s).to_path_buf())
                        .unwrap_or_else(|| current_dir().unwrap());

    // Initialize logger...
    initialize_logger().unwrap();

    info!("Starting...");
    info!("store-latest-only: {}", store_latest_only);
    info!("force: {}", force);
    info!("output_dir: {}", output_dir.display());
    
    let result = main_impl(store_latest_only, force, output_dir);

    match result {
        Ok(_) => {
            info!("Done");
        },
        Err(app_err) => {
            error!("Failed: {}", app_err);
            exit(1);
        }
    }
}

fn main_impl (store_latest_only: bool, force: bool, output_dir: PathBuf) -> Result<(), AppErr> {

    // Download and parse the "latest.json" metadata
    let cache_buster = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let url = format!("http://himawari8-dl.nict.go.jp/himawari8/img/D531106/latest.json?uid={}", cache_buster);

    let mut response = reqwest::get(&url)?;
    if !response.status().is_success() {
        error!("Unable to download latest.json: {}", response.status());
        exit(1);
    }

    let mut json_content = String::new();
    response.read_to_string(&mut json_content)?;

    let latest_info = serde_json::from_str::<LatestInfo>(&json_content)?;
    let latest_date = Utc.datetime_from_str(&latest_info.date, "%Y-%m-%d %H:%M:%S")?;

    let width = 550;
    let level = 4; // Level can be 4, 8, 16, 20
    let time  = latest_date.format("%H%M%S");
    let year  = latest_date.format("%Y");
    let month = latest_date.format("%m");
    let day   = latest_date.format("%d");

    // Create the output folder if it doesnt exist (e.g. "My Pictures\Himawari\")
    if !output_dir.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(&output_dir)?;
    }

    info!("Writing images to {}", output_dir.display());

    let mut output_file_path = output_dir.clone();

    // The filename that will be written
    // NOTE: Output format detemined by file extension (jpeg or png)
    if store_latest_only {
        output_file_path.push("himawari8_latest.jpeg");
    } else {
        output_file_path.push(format!("himawari8_{}{}{}_{}.jpeg", year, month, day, time));
    }

    // Have we already downloaded this one?
    if !store_latest_only && !force && output_file_path.exists() {
        error!("Output file {} already exists. Use --force to overwrite", output_file_path.display());
        exit(1);
    }

    // Output buffer
    let mut canvas = ImageBuffer::new(width * level, width * level);

    // Download each image into a temporary buffer and copy it into the buffer
    for y in 0..level {
        for x in 0..level {

            let url = format!(
                "http://himawari8-dl.nict.go.jp/himawari8/img/D531106/{level}d/{width}/{year}/{month}/{day}/{time}_{x}_{y}.png", 
                level = level, width = width, year = year, month = month, day = day, time = time, x = x, y = y
            );

            info!("Downloading chunk {}...", url);

            let mut response = reqwest::get(&url)?;
            if !response.status().is_success() {
                warn!("Unable to download chunk: {}", response.status());
                continue;
            }

            let mut image_data = Vec::new();
            response.read_to_end(&mut image_data)?;

            let block = load_from_memory_with_format(&image_data, ImageFormat::PNG)?;
            canvas.copy_from(&block, x * width, y * width);
        }
    }

    info!("Writing out to {}", output_file_path.display());
    canvas.save(output_file_path.as_path())?;

    info!("Setting wallpaper...");
    set_wallpaper_registry_keys(output_file_path.as_path())?;
    user32_set_desktop_wallpaper(output_file_path.as_path());

    Ok(())
}

fn set_wallpaper_registry_keys (image_path: &Path) -> Result<(), AppErr> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_desktop = hkcu.open_subkey_with_flags("Control Panel\\Desktop", KEY_WRITE)?;
    key_desktop.set_value("Wallpaper", &image_path.as_os_str())?;
    key_desktop.set_value("WallpaperStyle", &"6")?;
    key_desktop.set_value("TileWallpaper", &"0")?;
    
    let key_colors = hkcu.open_subkey_with_flags("Control Panel\\Colors", KEY_WRITE)?;
    key_colors.set_value("Background", &"0 0 0")?;

    Ok(())
}

fn user32_set_desktop_wallpaper (image_path: &Path) {
    use std::ffi::{CString};
    use user32::{SystemParametersInfoA};
    use winapi::{c_void};
    use winapi::winuser::{SPI_SETDESKWALLPAPER};
    
    let path_ptr = CString::new(image_path.to_str().unwrap()).unwrap().into_raw();

    // NOTE: SystemParametersInfoW is apparently the 64-bit call, but appears to only set the desktop background to black?
    // let system_parameters_info = if cfg!(target_pointer_width = "32") { SystemParametersInfoA } else { SystemParametersInfoW };

    let ok = unsafe { SystemParametersInfoA(SPI_SETDESKWALLPAPER, 0, path_ptr as *mut c_void, 0) };

    unsafe { CString::from_raw(path_ptr) };

    if ok == 0 {
        warn!("Unable to set desktop background?");
    }
}