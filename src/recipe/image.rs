use anyhow::Result;
use std::env;

use camino::Utf8PathBuf;
use clap::Args;
use reqwest::Url;
use std::io::Cursor;

use crate::Context;

#[derive(Debug, Args)]
pub struct ImageArgs {
    /// Recipe file for which to download image
    input: Utf8PathBuf,
}

const ACCESS_KEY: &str = "COOK_UNSPLASH_ACCESS_KEY";

pub fn run(_ctx: &Context, args: ImageArgs) -> Result<()> {
    let recipe_name = args.input.file_stem().unwrap();

    let access_key = env::var(ACCESS_KEY).expect("Could not find COOK_UNSPLASH_ACCESS_KEY environment variable, please register for free at https://unsplash.com/documentation#registering-your-application and set environment variable.");

    let url = format!("https://api.unsplash.com/photos/random?query={recipe_name}&orientation=landscape&client_id={access_key}");

    let json: serde_json::Value = reqwest::blocking::get(url)
        .expect("Could not download image location from Unsplash. Make sure you have access to the internet and valid client_id.")
        .json()
        .expect("Invalid JSON response from Unsplash. Try again to look for a new random image.");

    let image_url: &str = json
        .get("urls")
        .unwrap()
        .get("regular")
        .expect("Unexpected JSON response structure for random image")
        .as_str()
        .unwrap();

    let image_url = Url::parse(image_url).expect("Can't parse URL");

    let output_path = args
        .input
        .parent()
        .unwrap()
        .join(format!("{recipe_name}.jpg"));

    println!("Saving image to {}", output_path);

    let response = reqwest::blocking::get(image_url)?.bytes()?;
    let mut file = std::fs::File::create(output_path).expect("failed to copy content");
    let mut content = Cursor::new(response);

    std::io::copy(&mut content, &mut file)?;

    Ok(())
}
