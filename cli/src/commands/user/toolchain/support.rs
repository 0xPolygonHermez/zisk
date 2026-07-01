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

/// Major version of the ZisK toolchain to track. When no explicit version is
/// requested, the installer downloads the highest `zisk-<TOOLCHAIN_MAJOR>.x.y`
/// release published for `0xPolygonHermez/rust` (greatest minor, then patch),
/// instead of whatever `releases/latest` happens to point at. Bump this when the
/// toolchain moves to a new incompatible major.
const TOOLCHAIN_MAJOR: u64 = 1;

/// From a list of git tag names, pick the highest `zisk-<major>.x.y` — greatest
/// minor, then greatest patch. Tags that don't match the exact
/// `zisk-<major>.<minor>.<patch>` shape (all numeric) are ignored. Returns the
/// full tag, e.g. `zisk-1.4.2`.
fn select_latest_tag(tags: &[String], major: u64) -> Option<String> {
    tags.iter()
        .filter_map(|tag| {
            let version = tag.strip_prefix("zisk-")?;
            let mut parts = version.split('.');
            let maj: u64 = parts.next()?.parse().ok()?;
            let min: u64 = parts.next()?.parse().ok()?;
            let patch: u64 = parts.next()?.parse().ok()?;
            // Reject extra components (e.g. `zisk-1.2.3.4`) and other majors.
            if parts.next().is_some() || maj != major {
                return None;
            }
            Some((min, patch, tag.clone()))
        })
        .max_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)))
        .map(|(_, _, tag)| tag)
}

/// Fetch the `tag_name`s of all releases published for `0xPolygonHermez/rust`.
/// The fork only publishes ZisK toolchain releases, so a single page (100) is
/// plenty and keeps us clear of the thousands of upstream Rust tags.
async fn fetch_release_tags(client: &Client) -> Result<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
    }

    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("cargo-zisk"));
    headers.insert("Accept", HeaderValue::from_static("application/vnd.github+json"));

    let releases: Vec<Release> = client
        .get("https://api.github.com/repos/0xPolygonHermez/rust/releases?per_page=100")
        .headers(headers)
        .send()
        .await
        .context("querying GitHub releases")?
        .error_for_status()
        .context("GitHub releases request failed")?
        .json()
        .await
        .context("parsing GitHub releases response")?;

    Ok(releases.into_iter().map(|r| r.tag_name).collect())
}

/// Build the GitHub release URL for the toolchain tarball.
///
/// With an explicit `version` the URL points at that exact release. Otherwise we
/// resolve the highest `zisk-<TOOLCHAIN_MAJOR>.x.y` release via the GitHub API
/// (rather than `releases/latest`, which could jump to a newer major).
pub(crate) async fn get_toolchain_download_url(
    client: &Client,
    target: &String,
    version: &Option<String>,
) -> Result<String> {
    let tag = if let Some(version) = version {
        version.clone()
    } else {
        let tags = fetch_release_tags(client).await?;
        select_latest_tag(&tags, TOOLCHAIN_MAJOR).with_context(|| {
            format!(
                "no `zisk-{TOOLCHAIN_MAJOR}.x.y` toolchain release found for 0xPolygonHermez/rust"
            )
        })?
    };

    Ok(format!(
        "https://github.com/0xPolygonHermez/rust/releases/download/{tag}/rust-toolchain-{target}.tar.gz",
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_target_is_a_triple() {
        let t = get_target();
        assert!(!t.is_empty());
        assert!(t.contains('-'), "expected a target triple, got {t}");
    }

    #[test]
    fn is_supported_target_matches_host_cfg() {
        let expected = cfg!(any(
            all(target_arch = "x86_64", target_os = "linux"),
            all(target_arch = "aarch64", target_os = "macos")
        ));
        assert_eq!(is_supported_target(), expected);
    }

    #[tokio::test]
    async fn download_url_pinned_version() {
        // The pinned-version branch never touches the network, so any client works.
        let client = Client::new();
        let url = get_toolchain_download_url(
            &client,
            &"x86_64-unknown-linux-gnu".to_string(),
            &Some("v1.2.3".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(
            url,
            "https://github.com/0xPolygonHermez/rust/releases/download/v1.2.3/rust-toolchain-x86_64-unknown-linux-gnu.tar.gz"
        );
    }

    #[test]
    fn select_latest_tag_picks_highest_minor_then_patch() {
        let tags = vec![
            "zisk-1.0.0".to_string(),
            "zisk-1.2.9".to_string(),
            "zisk-1.10.0".to_string(),  // higher minor than 1.2.x
            "zisk-1.10.3".to_string(),  // highest patch of the highest minor
            "zisk-2.5.0".to_string(),   // different major, ignored
            "1.99.0".to_string(),       // missing prefix, ignored
            "zisk-1.4".to_string(),     // too few components, ignored
            "zisk-1.4.2.1".to_string(), // too many components, ignored
        ];
        assert_eq!(select_latest_tag(&tags, 1).as_deref(), Some("zisk-1.10.3"));
        assert_eq!(select_latest_tag(&tags, 2).as_deref(), Some("zisk-2.5.0"));
        assert_eq!(select_latest_tag(&tags, 3), None);
    }
}
