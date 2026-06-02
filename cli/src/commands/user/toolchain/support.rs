//! Shared plumbing for the toolchain `build`/`install` subcommands: host-target
//! detection, release-URL construction, a progress-bar downloader, and a small
//! `Command` runner.

use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};

/// Run a [`Command`] inheriting stdio, turning a spawn/IO failure into an error.
pub(crate) trait CommandExecutor {
    fn run(&mut self) -> Result<()>;
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

/// Host target triple (e.g. `x86_64-unknown-linux-gnu`).
pub(crate) fn get_target() -> String {
    target_lexicon::HOST.to_string()
}

/// Whether a prebuilt ZisK toolchain is published for the current host.
#[allow(unreachable_code)]
pub(crate) fn is_supported_target() -> bool {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    return true;

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    return true;

    false
}

/// Build the GitHub release URL for the toolchain tarball.
pub(crate) async fn get_toolchain_download_url(
    target: &String,
    version: &Option<String>,
) -> String {
    if let Some(version) = version {
        format!(
        "https://github.com/0xPolygonHermez/rust/releases/download/{version}/rust-toolchain-{target}.tar.gz",
    )
    } else {
        format!(
        "https://github.com/0xPolygonHermez/rust/releases/latest/download/rust-toolchain-{target}.tar.gz",
    )
    }
}

/// Stream-download `url` into `file`, rendering a progress bar.
pub(crate) async fn download_file(
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
