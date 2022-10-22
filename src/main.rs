// NOTE: Set "windows" subsystem for release builds
// This disables console output, which prevents a console window from opening and stealing focus when running this program as a scheduled task.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate clap;
extern crate image;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate log;
extern crate rayon;
extern crate simple_logging;

#[cfg(windows)]
extern crate winapi;
#[cfg(windows)]
extern crate winreg;

mod error;
mod margins;
mod output_format;
mod output_level;
#[cfg(windows)]
mod ffi_windows;
#[cfg(not(windows))]
mod ffi_unix;

use std::env::current_dir;
use std::fs::DirBuilder;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::offset::Utc;
use chrono::prelude::*;
use image::{load_from_memory_with_format, GenericImage, ImageBuffer, ImageFormat};
use clap::{App, Arg};
use rayon::prelude::*;

use self::error::AppErr;
use self::margins::Margins;
use self::output_format::OutputFormat;
use self::output_level::OutputLevel;
#[cfg(windows)]
use self::ffi_windows::set_wallpaper;
#[cfg(not(windows))]
use self::ffi_unix::set_wallpaper;

#[cfg(debug_assertions)]
fn initialize_logger() -> io::Result<()> {
    simple_logging::log_to_stderr(log::LogLevelFilter::Info)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

#[cfg(not(debug_assertions))]
fn initialize_logger() -> io::Result<()> {
    simple_logging::log_to_file("himawari-desktop-updater.log", log::LogLevelFilter::Info)
}

fn main() {
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
                .help("If set, writes the output to a single file named 'latest'"))

            .arg(Arg::with_name("force")
                .long("force")
                .help("If set, allow the output file to be overwritten"))

            .arg(Arg::with_name("set-wallpaper")
                .long("set-wallpaper")
                .help("If set, attempts to set the current user's desktop background to the output image"))

            .arg(Arg::with_name("output-dir")
                .long("output-dir")
                .help("Set the output directory")
                .required(true)
                .value_name("OUTPUT_DIR"))

            .arg(Arg::with_name("output-format")
                .long("output-format")
                .help("Set the output format, PNG or JPEG (default)")
                .validator(|s| {
                    OutputFormat::parse(&s)
                        .map(|_| ())
                        .map_err(|err| err.to_string())
                })
                .value_name("OUTPUT_FORMAT"))

            .arg(Arg::with_name("output-level")
                .long("output-level")
                .help("Set the dimensions of the output image: 4, 8, 16 or 20. ")
                .validator(|s| {
                    OutputLevel::parse(&s)
                        .map(|_| ())
                        .map_err(|err| err.to_string())
                })
                .value_name("OUTPUT_LEVEL"))

            .arg(Arg::with_name("margins")
                .long("margins")
                .help("Set top,right,bottom,left margins on the output image")
                .validator(|s| {
                    Margins::parse(&s)
                        .map(|_| ())
                        .map_err(|err| err.to_string())
                })
                .value_name("TOP,RIGHT,BOTTOM,LEFT"))

            .get_matches();

    // If set, write only to "latest.png"
    let store_latest_only = args.is_present("store-latest-only");

    // If set, overwrite output image
    let force = args.is_present("force");

    // Try to set the desktop background?
    let try_set_wallpaper = args.is_present("set-wallpaper");

    // Directory to write images out to
    let output_dir = args
        .value_of("output-dir")
        .map(|s| {
            let mut path = current_dir().unwrap();
            path.push(s);
            path
        })
        .unwrap();

    // Optional output image format
    let output_format = args
        .value_of("output-format")
        .and_then(|s| OutputFormat::parse(s).ok())
        .unwrap_or_else(|| OutputFormat::default());

    // Optional output image resolution
    let output_level = args
        .value_of("output-level")
        .and_then(|s| OutputLevel::parse(s).ok())
        .unwrap_or_else(|| OutputLevel::default());

    // Optional margins to put on the image
    let margins = args
        .value_of("margins")
        .and_then(|s| Margins::parse(s).ok())
        .unwrap_or_else(|| Margins::empty());

    info!("Starting...");
    info!("store-latest-only: {}", store_latest_only);
    info!("force: {}", force);
    info!("output-dir: {}", output_dir.display());
    info!("output-format: {}", output_format);
    info!("output-level: {}", output_level);
    info!(
        "margins: {}, {}, {}, {}",
        margins.top, margins.right, margins.bottom, margins.left
    );

    let result = download_latest_himawari_image(
        store_latest_only,
        force,
        margins,
        &output_dir,
        output_format,
        output_level,
    )
    .and_then(|image_path| {
        if try_set_wallpaper {
            set_wallpaper(&image_path)
        } else {
            Ok(())
        }
    });

    match result {
        Ok(()) => {
            info!("Done");
        }
        Err(app_err) => {
            error!("{}", app_err);
            exit(1);
        }
    }
}

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

fn download_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, AppErr> {
    let client = reqwest::blocking::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()?;
    let result: T = client.get(url).send()?.error_for_status()?.json()?;
    Ok(result)
}

fn download_bytes(url: &str) -> Result<Vec<u8>, AppErr> {
    let client = reqwest::blocking::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()?;
    let mut response = client.get(url).send()?.error_for_status()?;
    let mut data = Vec::new();
    response.read_to_end(&mut data)?;
    Ok(data)
}

#[derive(Serialize, Deserialize, Debug)]
struct LatestInfo {
    date: String,
    file: String,
}

fn download_latest_himawari_image(
    store_latest_only: bool,
    force: bool,
    margins: Margins,
    output_dir: &Path,
    output_format: OutputFormat,
    output_level: OutputLevel,
) -> Result<PathBuf, AppErr> {
    // Prepare the output folder
    info!("Preparing output dir...");
    if !output_dir.exists() {
        DirBuilder::new().recursive(true).create(&output_dir)?;
    }

    const HIMAWARI_BASE_URL: &'static str = "https://himawari8-dl.nict.go.jp/himawari8/img/D531106";

    // Download and parse the "latest.json" metadata
    let cache_buster = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    info!("Downloading latest metadata...");
    let url = format!("{}/latest.json?_={}", HIMAWARI_BASE_URL, cache_buster);

    let latest_info: LatestInfo = download_json(&url)?;
    let latest_date = Utc.datetime_from_str(&latest_info.date, "%Y-%m-%d %H:%M:%S")?;

    info!("Latest image available: {}", latest_date);

    // Width and Level determine the dimensions and count of image fragments downloaded
    let width = 550;
    // Level can be 4, 8, 16, 20
    let level = output_level.to_level();
    let time = latest_date.format("%H%M%S");
    let year = latest_date.format("%Y");
    let month = latest_date.format("%m");
    let day = latest_date.format("%d");

    // The filename that will be written
    let mut output_file_path = output_dir.to_path_buf();
    if store_latest_only {
        output_file_path.push(format!("himawari8_latest.{}", output_format));
    } else {
        output_file_path.push(format!(
            "himawari8_{}{}{}_{}.{}",
            year, month, day, time, output_format
        ));
    }

    // Have we already downloaded this one?
    if output_file_path.exists() && !store_latest_only && !force {
        warn!(
            "Output file {} already exists. Use --force to overwrite",
            output_file_path.display()
        );
        return Ok(output_file_path);
    }

    // For each (x, y) position in a level*level image...
    let chunk_positions: Vec<_> = (0..level)
        .flat_map(|y| (0..level).map(move |x| (x, y)))
        .collect();

    // In parallel, download each chunk into memory
    let chunks: Vec<_> = chunk_positions
        .into_par_iter()
        .filter_map(|(x, y)| {
            let url = format!(
                "{}/{}d/{}/{}/{}/{}/{}_{}_{}.png",
                HIMAWARI_BASE_URL, level, width, year, month, day, time, x, y
            );
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
    let w = margins.left + (width * level) + margins.right;
    let h = margins.top + (width * level) + margins.bottom;

    let mut canvas = ImageBuffer::new(w, h);

    for (x, y, image_data) in chunks {
        let block = load_from_memory_with_format(&image_data, ImageFormat::PNG)?;
        let x = margins.left + (x * width);
        let y = margins.top + (y * width);
        canvas.copy_from(&block, x, y);
    }

    // NOTE: Output format detemined by file extension (jpeg or png)
    info!("Writing out to {}", output_file_path.display());
    canvas.save(output_file_path.as_path())?;

    Ok(output_file_path)
}