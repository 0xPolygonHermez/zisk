pub mod commands;
pub mod toolchain;
pub mod ux;

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
    time::Duration,
};
use tokio::time::sleep;

pub const RUSTUP_TOOLCHAIN_NAME: &str = "zisk";

pub const ZISK_VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_BUILD_TIMESTAMP"),
    ")"
);

const ZISK_TARGET: &str = "riscv64ima-zisk-zkvm-elf";

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
    let max_retries = 3;
    let delay = Duration::from_secs(3);

    for attempt in 1..=max_retries {
        if let Ok(response) = client.head(url).send().await {
            if response.status().is_success() {
                return true;
            }
        }

        // If the request failed, wait for 3 seconds before retrying
        if attempt < max_retries {
            sleep(delay).await;
        }
    }

    false
}

#[allow(unreachable_code)]
pub fn is_supported_target() -> bool {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    return true;

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    return true;

    false
}

pub async fn get_toolchain_download_url(target: String) -> String {
    format!(
        "https://github.com/0xPolygonHermez/rust/releases/latest/download/rust-toolchain-{target}.tar.gz",
    )
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
    println!("Downloading {url}");

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err("Error while downloading file"))?;
        file.write_all(&chunk).or(Err("Error while writing to file"))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    let msg = format!("Downloaded {url} to {file:?}");
    pb.finish_with_message(msg);
    Ok(())
}
