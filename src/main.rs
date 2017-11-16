#![windows_subsystem = "windows"]

extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate image;

use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Read;
use std::path::Path;

use chrono::prelude::*;
use chrono::offset::{Utc};

use image::{GenericImage, ImageBuffer, ImageFormat, Rgba, load_from_memory_with_format};

#[derive(Serialize, Deserialize, Debug)]
struct LatestInfo {
    date: String,
    file: String
}

fn main() {
    
    // TODO:
    // --force
    // --store-latest-only

    // let force = true;
    let store_latest_only = true;

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

    // Create the folder My Pictures\Himawari\ if it doesnt exist
    // TODO

    // The filename that will be written
    let outfile =
        if store_latest_only {
            format!("latest.jpg")
        } else {
            format!("{}{}{}_{}.jpg", year, month, day, time)
        };

    let outfile = Path::new(&outfile);

    // Have we already downloaded this one?
    // if !store_latest_only && !force && file_exists(&outfile) {
    //     return;
    // }

    let mut canvas: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width * level, width * level);

    for y in 0..level {
        for x in 0..level {

            let block_url = format!("http://himawari8-dl.nict.go.jp/himawari8/img/D531106/{}d/{}/{}/{}/{}/{}_{}_{}.png", level, width, year, month, day, time, x, y);

            print!("Downloading {}...", block_url);

            let mut response = reqwest::get(&block_url).unwrap();
            assert!(response.status().is_success(), "Request {} failed with {}", block_url, response.status());

            let mut image_data = Vec::new();
            response.read_to_end(&mut image_data).unwrap();

            let block = load_from_memory_with_format(&image_data, ImageFormat::PNG).unwrap();

            canvas.copy_from(&block, x * width, y * width);

            println!(" Done");
        }
    }

    println!("Writing out to {:?}", outfile);
    canvas.save(outfile).unwrap();
}
