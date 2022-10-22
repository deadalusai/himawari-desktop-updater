// NOTE: Set "windows" subsystem for release builds
// This disables console output, which prevents a console window from opening and stealing focus when running this program as a scheduled task.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod error;
#[cfg(not(windows))]
mod ffi_unix;
#[cfg(windows)]
mod ffi_windows;
mod margins;
mod output_format;
mod output_level;

use std::env::current_dir;
use std::fs::DirBuilder;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::offset::Utc;
use chrono::prelude::*;
use image::{load_from_memory_with_format, GenericImage, ImageBuffer, ImageFormat};
use log::{error, info, warn};
use rayon::prelude::*;
use serde_derive::Deserialize;

use self::error::AppErr;
#[cfg(not(windows))]
use self::ffi_unix::set_wallpaper;
#[cfg(windows)]
use self::ffi_windows::set_wallpaper;
use self::margins::{Margins, MarginsValueParser};
use self::output_format::{OutputFormat, OutputFormatValueParser};
use self::output_level::{OutputLevel, OutputLevelValueParser};

fn make_clap_command() -> clap::Command {
    use clap::{Arg, ArgAction, Command};
    Command::new("himawari-desktop-updater")
        .version("0.1")
        .about("Downloads the latest photo from the Himawari-8 geo-synchronous satellite and sets it as your desktop background.")
        .author("Benjamin Fox")

        .arg(Arg::new("store-latest-only")
            .long("store-latest-only")
            .help("If set, writes the output to a single file named 'latest'")
            .action(ArgAction::SetTrue))

        .arg(Arg::new("force")
            .long("force")
            .help("If set, allow the output file to be overwritten")
            .action(ArgAction::SetTrue))

        .arg(Arg::new("set-wallpaper")
            .long("set-wallpaper")
            .help("If set, attempts to set the current user's desktop background to the output image")
            .action(ArgAction::SetTrue))

        .arg(Arg::new("output-dir")
            .long("output-dir")
            .help("Set the output directory")
            .required(true)
            .value_name("OUTPUT_DIR"))

        .arg(Arg::new("output-format")
            .long("output-format")
            .help("Set the output format")
            .value_name("OUTPUT_FORMAT")
            .value_parser(OutputFormatValueParser))

        .arg(Arg::new("output-level")
            .long("output-level")
            .help("Set the dimensions of the output image: 4, 8, 16 or 20. ")
            .value_name("OUTPUT_LEVEL")
            .value_parser(OutputLevelValueParser))

        .arg(Arg::new("margins")
            .long("margins")
            .help("Set top,right,bottom,left margins on the output image")
            .value_name("TOP,RIGHT,BOTTOM,LEFT")
            .value_parser(MarginsValueParser))
}

#[cfg(debug_assertions)]
fn initialize_logger() {
    simple_logging::log_to_stderr(log::LevelFilter::Info);
}

#[cfg(not(debug_assertions))]
fn initialize_logger() {
    // In release builds the program runs as a "headless" application (under Windows) so redirect logs to file
    simple_logging::log_to_file("himawari-desktop-updater.log", log::LevelFilter::Info)
        .expect("Failed to open log file for writing");
}

#[cfg(debug_assertions)]
fn print_clap_err(e: clap::error::Error) {
    e.print().unwrap()
}

#[cfg(not(debug_assertions))]
fn print_clap_err(e: clap::error::Error) {
    // NOTE: In Release mode the program is headless (under windows) so there so print help to the log stream which will
    // redirect it to the right place.
    error!("{}", e);
}

fn main() {
    // Initialize logger...
    initialize_logger();

    let args = match make_clap_command().try_get_matches() {
        Err(e) => {
            print_clap_err(e);
            return;
        }
        Ok(args) => args,
    };

    // If set, write only to "latest.png"
    let store_latest_only = args.get_flag("store-latest-only");

    // If set, overwrite output image
    let force = args.get_flag("force");

    // Try to set the desktop background?
    let try_set_wallpaper = args.get_flag("set-wallpaper");

    // Directory to write images out to
    let output_dir = args
        .get_one::<String>("output-dir")
        .map(|s| {
            let mut path = current_dir().unwrap();
            path.push(s);
            path
        })
        .unwrap();

    // Optional output image format
    let output_format = args
        .get_one::<OutputFormat>("output-format")
        .cloned()
        .unwrap_or_default();

    // Optional output image resolution
    let output_level = args
        .get_one::<OutputLevel>("output-level")
        .cloned()
        .unwrap_or_default();

    // Optional margins to put on the image
    let margins = args
        .get_one::<Margins>("margins")
        .cloned()
        .unwrap_or_default();

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

#[derive(Deserialize, Debug)]
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
    let cache_buster = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    info!("Downloading latest metadata...");
    let url = format!("{}/latest.json?_={}", HIMAWARI_BASE_URL, cache_buster);

    let latest_info: LatestInfo = download_json(&url)?;
    let latest_date = Utc.datetime_from_str(&latest_info.date, "%Y-%m-%d %H:%M:%S")?;

    info!(
        "Latest image available is {} with timestamp {}",
        latest_info.file, latest_date
    );

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

    let download_chunk = |x: u32, y: u32| -> Result<image::DynamicImage, AppErr> {
        let url = format!(
            "{}/{}d/{}/{}/{}/{}/{}_{}_{}.png",
            HIMAWARI_BASE_URL, level, width, year, month, day, time, x, y
        );
        info!("Downloading chunk {}...", url);
        let image = download_bytes(&url)?;
        let image = load_from_memory_with_format(&image, ImageFormat::Png)?;
        Ok(image)
    };

    // In parallel, download each chunk into memory
    let chunks: Vec<_> = chunk_positions
        .into_par_iter()
        .filter_map(|(x, y)| match download_chunk(x, y) {
            Ok(c) => Some((x, y, c)),
            Err(err) => {
                // For now, just leave a hole in the final image
                warn!("{}", err);
                None
            }
        })
        .collect();

    info!("Combining chunks...");
    let w = margins.left + (width * level) + margins.right;
    let h = margins.top + (width * level) + margins.bottom;

    let mut buf = ImageBuffer::new(w, h);

    for (x, y, chunk) in chunks {
        let x = margins.left + (x * width);
        let y = margins.top + (y * width);
        buf.copy_from(&chunk, x, y)?;
    }

    // NOTE: Output format detemined by file extension (jpeg or png)
    info!("Writing out to {}", output_file_path.display());
    buf.save(output_file_path.as_path())?;

    Ok(output_file_path)
}
