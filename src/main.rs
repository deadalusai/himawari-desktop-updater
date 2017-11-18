// NOTE: Set "windows" subsystem for release builds
// This disables console output, which prevents a console window from opening and stealing focus when running this program as a scheduled task. 
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
extern crate rayon;

pub mod app_error;

use std::time::{SystemTime, UNIX_EPOCH};
use std::io::{Read};
use std::fs::{DirBuilder};
use std::env::{current_dir};
use std::process::{exit};
use std::path::{Path, PathBuf};
use std::io;

use chrono::prelude::*;
use chrono::offset::{Utc};

use image::{GenericImage, ImageBuffer, ImageFormat, load_from_memory_with_format};

use clap::{App, Arg};

use app_error::{AppErr};

use rayon::prelude::*;

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

    // Initialize logger...
    initialize_logger().unwrap();
    
    // NOTE: Args are still parsed in Release mode but the program is headless so there is no way to print out help
    let args = 
        App::new("himawari-desktop-updater")
            .version("0.1")
            .about("Downloads the latest photo from the Himawari-8 geo-synchronous satellite and sets it as your desktop background.")
            .author("Benjamin Fox")

            .arg(Arg::with_name("store-latest-only")
                .long("store-latest-only")
                .help("If set, writes the output to a single file named 'latest"))

            .arg(Arg::with_name("force")
                .long("force")
                .help("If set, allow the output file to be overwritten"))

            .arg(Arg::with_name("output-dir")
                .long("output-dir")
                .help("Set the output directory")
                .required(true)
                .value_name("OUTPUT_DIR"))

            .get_matches();

    // If set, write only to "latest.png"
    let store_latest_only = args.is_present("store-latest-only");

    // If set, overwrite output image
    let force = args.is_present("force");

    // Directory to write images out to
    let output_dir = match args.value_of("output-dir") {
        Some(s) => {
            let mut path = current_dir().unwrap();
            path.push(s);
            path
        },
        None => unreachable!()
    };

    info!("Starting...");
    info!("store-latest-only: {}", store_latest_only);
    info!("force: {}", force);
    info!("output-dir: {}", output_dir.display());
    
    let result =
        download_latest_himawari_image(store_latest_only, force, &output_dir)
            .and_then(|image_path| set_wallpaper(&image_path));

    match result {
        Ok(()) => {
            info!("Done");
        },
        Err(app_err) => {
            error!("{}", app_err);
            exit(1);
        }
    }
}

fn download_string (url: &str) -> Result<String, AppErr> {
    let mut response = reqwest::get(url)?.error_for_status()?;
    let mut content = String::new();
    response.read_to_string(&mut content)?;
    Ok(content)
}

fn download_bytes (url: &str) -> Result<Vec<u8>, AppErr> {
    let mut response = reqwest::get(url)?.error_for_status()?;
    let mut data = Vec::new();
    response.read_to_end(&mut data)?;
    Ok(data)
}

#[derive(Serialize, Deserialize, Debug)]
struct LatestInfo {
    date: String,
    file: String
}

fn download_latest_himawari_image (store_latest_only: bool, force: bool, output_dir: &Path) -> Result<PathBuf, AppErr> {

    // Prepare the output folder
    info!("Preparing output dir...");
    if !output_dir.exists() {
        DirBuilder::new()
            .recursive(true)
            .create(&output_dir)?;
    }

    const HIMAWARI_BASE_URL: &'static str = "http://himawari8-dl.nict.go.jp/himawari8/img/D531106";

    // Download and parse the "latest.json" metadata
    let cache_buster = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    info!("Downloading latest metadata...");
    let url = format!("{}/latest.json?uid={}", HIMAWARI_BASE_URL, cache_buster);

    let json_content = download_string(&url)?;

    let latest_info = serde_json::from_str::<LatestInfo>(&json_content)?;
    let latest_date = Utc.datetime_from_str(&latest_info.date, "%Y-%m-%d %H:%M:%S")?;

    info!("Latest image available: {}", latest_date);

    let width = 550;
    let level = 4; // Level can be 4, 8, 16, 20
    let time  = latest_date.format("%H%M%S");
    let year  = latest_date.format("%Y");
    let month = latest_date.format("%m");
    let day   = latest_date.format("%d");

    // The filename that will be written
    let mut output_file_path = output_dir.to_path_buf();
    if store_latest_only {
        output_file_path.push("himawari8_latest.jpeg");
    } else {
        output_file_path.push(format!("himawari8_{}{}{}_{}.jpeg", year, month, day, time));
    }

    // Have we already downloaded this one?
    if output_file_path.exists() && !store_latest_only && !force {
        warn!("Output file {} already exists. Use --force to overwrite", output_file_path.display());
        return Ok(output_file_path);
    }

    // For each (x, y) position in a level*level image...
    let chunk_positions: Vec<_> =
        (0..level).flat_map(|y| (0..level).map(move |x| (x, y)))
        .collect();

    // In parallel, download each chunk into memory
    let chunks: Vec<_> = 
        chunk_positions
        .into_par_iter()
        .filter_map(|(x, y)| {
            let url = format!("{}/{}d/{}/{}/{}/{}/{}_{}_{}.png", HIMAWARI_BASE_URL, level, width, year, month, day, time, x, y);
            info!("Downloading chunk {}...", url);
            match download_bytes(&url) {
                Ok(image_data) => Some((x, y, image_data)),
                Err(err) => {
                    // For now, just leave a hole in the final image
                    warn!("{}", err);
                    None
                }
            }            
        })
        .collect();

    info!("Combining chunks...");
    let mut canvas = ImageBuffer::new(width * level, width * level);

    for (x, y, image_data) in chunks {
        let block = load_from_memory_with_format(&image_data, ImageFormat::PNG)?;
        canvas.copy_from(&block, x * width, y * width);
    }

    // NOTE: Output format detemined by file extension (jpeg or png)
    info!("Writing out to {}", output_file_path.display());
    canvas.save(output_file_path.as_path())?;

    Ok(output_file_path)
}

// TODO: Linux/OSX versions of set_wallpaper?
fn set_wallpaper (image_path: &Path) -> Result<(), AppErr> {
    // Set registry flags to control wallpaper style
    info!("Setting Windows desktop wallpaper registry keys");

    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_desktop = hkcu.open_subkey_with_flags("Control Panel\\Desktop", KEY_WRITE)?;
    key_desktop.set_value("Wallpaper", &image_path.as_os_str())?; // Wallpaper path
    key_desktop.set_value("WallpaperStyle", &"6")?; // Style "Fit"
    key_desktop.set_value("TileWallpaper", &"0")?; // Tiling disabled

    let key_colors = hkcu.open_subkey_with_flags("Control Panel\\Colors", KEY_WRITE)?;
    key_colors.set_value("Background", &"0 0 0")?; // Black background

    // Also set wallpaper and fill color through user32 API
    info!("Setting Windows desktop wallpaper");
    
    use std::os::windows::ffi::{OsStrExt};
    use std::iter::once;
    use user32::{SystemParametersInfoW, SetSysColors};
    use winapi::winnt::{PVOID};
    use winapi::winuser::{SPI_SETDESKWALLPAPER, COLOR_BACKGROUND};

    // Desktop wallpaper
    unsafe {
        // NUL-terminated unicode string
        let path_unicode: Vec<u16> = image_path.as_os_str().encode_wide().chain(once(0)).collect();
        SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, path_unicode.as_ptr() as PVOID, 0);
    }

    // Background fill (black)
    unsafe { 
        SetSysColors(1, [COLOR_BACKGROUND].as_ptr(), [0, 0, 0].as_ptr());
    }
    
    Ok(())
}