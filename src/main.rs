// #![windows_subsystem = "windows"]

extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate image;
extern crate clap;

use std::time::{SystemTime, UNIX_EPOCH};
use std::io::{Read};
use std::env::{current_dir};
use std::fs::{DirBuilder};
use std::process::{exit};
use std::path::{Path};

use chrono::prelude::*;
use chrono::offset::{Utc};

use image::{GenericImage, ImageBuffer, ImageFormat, load_from_memory_with_format};

use clap::{App, Arg};

#[derive(Serialize, Deserialize, Debug)]
struct LatestInfo {
    date: String,
    file: String
}

fn main() {
    
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

    // Download and parse the "latest.json" metadata
    let cache_buster = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let url = format!("http://himawari8-dl.nict.go.jp/himawari8/img/D531106/latest.json?uid={}", cache_buster);

    let mut response = reqwest::get(&url).unwrap();
    assert!(response.status().is_success(), "Request {} failed with {}", url, response.status());

    let mut json_content = String::new();
    response.read_to_string(&mut json_content).unwrap();

    let latest_info: LatestInfo = serde_json::from_str(&json_content).unwrap();

    let latest_date = Utc.datetime_from_str(&latest_info.date, "%Y-%m-%d %H:%M:%S").unwrap();

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
            .create(&output_dir)
            .unwrap();
    }

    let mut output_file_path = output_dir.clone();

    // The filename that will be written
    if store_latest_only {
        output_file_path.push("latest.png");
    } else {
        output_file_path.push(format!("{}{}{}_{}.png", year, month, day, time));
    }

    println!("Writing output to {}", output_file_path.to_string_lossy());

    // Have we already downloaded this one?
    if !store_latest_only && !force && output_file_path.exists() {
        println!("Output file {} already exists. Use --force to overwrite", output_file_path.to_string_lossy());
        exit(1);
    }

    // Output buffer
    let mut canvas = ImageBuffer::new(width * level, width * level);

    // Download each image into a temporary buffer and copy it into the buffer
    for y in 0..level {
        for x in 0..level {

            let block_url = format!(
                "http://himawari8-dl.nict.go.jp/himawari8/img/D531106/{level}d/{width}/{year}/{month}/{day}/{time}_{x}_{y}.png", 
                level = level, width = width, year = year, month = month, day = day, time = time, x = x, y = y
            );

            println!("Downloading {}...", block_url);

            let mut response = reqwest::get(&block_url).unwrap();
            assert!(response.status().is_success(), "Request {} failed with {}", block_url, response.status());

            let mut image_data = Vec::new();
            response.read_to_end(&mut image_data).unwrap();

            let block = load_from_memory_with_format(&image_data, ImageFormat::PNG).unwrap();
            canvas.copy_from(&block, x * width, y * width);
        }
    }

    println!("Writing out to {}", output_file_path.to_string_lossy());
    canvas.save(output_file_path).unwrap();
}
