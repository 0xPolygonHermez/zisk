pub mod commands;
pub mod toolchain;

use anyhow::{Context, Result};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use std::{
    cmp::min,
    fs::File,
    io::Write,
    process::{Command, Stdio},
};

pub const RUSTUP_TOOLCHAIN_NAME: &str = "zisk";

pub const ZISK_VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_BUILD_TIMESTAMP"),
    ")"
);

const ZISK_TARGET: &str = "riscv64ima-polygon-ziskos-elf";

trait CommandExecutor {
    fn run(&mut self) -> Result<()>;
}

pub fn get_target() -> String {
    target_lexicon::HOST.to_string()
}

impl CommandExecutor for Command {
    fn run(&mut self) -> Result<()> {
        self.stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .output()
            .with_context(|| format!("while executing `{:?}`", &self))
            .map(|_| ())
    }
}
pub async fn url_exists(client: &Client, url: &str) -> bool {
    let res = client.head(url).send().await;
    res.is_ok()
}

#[allow(unreachable_code)]
pub fn is_supported_target() -> bool {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    return true;

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    return true;

    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    return true;

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    return true;

    false
}

pub async fn get_toolchain_download_url(client: &Client, target: String) -> String {
    // Get latest tag from https://api.github.com/repos/0xPolygonHermez/rust/releases/latest
    // and use it to construct the download URL.
    let url = "https://api.github.com/repos/0xPolygonHermez/rust/releases/latest";
    let json = client.get(url).send().await.unwrap().json::<serde_json::Value>().await.unwrap();

    let name: String = format!("rust-toolchain-{}.tar.gz", target);
    if let Some(assets) = json["assets"].as_array() {
        // Iterate over the array and extract the desired URL
        for asset in assets {
            if let Some(asset_name) = asset["name"].as_str() {
                if asset_name == name {
                    if let Some(url) = asset["url"].as_str() {
                        return url.to_string();
                    }
                }
            }
        }
    }
    "".to_string()
}

pub async fn download_file(
    client: &Client,
    url: &str,
    file: &mut File,
) -> std::result::Result<(), String> {
    let mut headers = HeaderMap::new();

    headers.insert("Accept", HeaderValue::from_static("application/octet-stream"));
    let res = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let total_size =
        res.content_length().ok_or(format!("Failed to get content length from '{}'", &url))?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").unwrap()
        .progress_chars("#>-"));
    println!("Downloading {}", url);

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err("Error while downloading file"))?;
        file.write_all(&chunk).or(Err("Error while writing to file"))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    let msg = format!("Downloaded {} to {:?}", url, file);
    pb.finish_with_message(msg);
    Ok(())
}
