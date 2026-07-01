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

/// GitHub API token from the environment, if set and non-empty. Sending it lifts
/// the anonymous rate limit (60 → 5000 req/h) and grants access to a private fork.
/// `GITHUB_TOKEN` takes precedence over `GH_TOKEN` (the `gh` CLI's variable).
fn github_token() -> Option<String> {
    std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .ok()
        .filter(|token| !token.is_empty())
}

/// Add `Authorization: Bearer <token>` to `headers` when a GitHub token is set in
/// the environment. No-op otherwise (requests stay anonymous).
fn add_github_auth(headers: &mut HeaderMap) {
    if let Some(token) = github_token() {
        if let Ok(value) = HeaderValue::from_str(&format!("Bearer {token}")) {
            headers.insert("Authorization", value);
        }
    }
}

/// Fetch the tag names under `refs/tags/zisk-<major>.` from `0xPolygonHermez/rust`
/// via the git matching-refs API. It returns only the ZisK tags of this major
/// (not the thousands of upstream Rust tags), and we assume each tag has a release
/// published under the same name.
async fn fetch_matching_tags(client: &Client, major: u64) -> Result<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct Ref {
        #[serde(rename = "ref")]
        name: String,
    }

    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("cargo-zisk"));
    headers.insert("Accept", HeaderValue::from_static("application/vnd.github+json"));
    add_github_auth(&mut headers);

    // The trailing `.` scopes the prefix to `zisk-<major>.` so `zisk-1.` never
    // matches `zisk-10.` (or the rolling `zisk-1-latest`, which has no dot).
    let url = format!(
        "https://api.github.com/repos/0xPolygonHermez/rust/git/matching-refs/tags/zisk-{major}."
    );

    let refs: Vec<Ref> = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .context("querying GitHub matching tags")?
        .error_for_status()
        .context("GitHub matching-refs request failed")?
        .json()
        .await
        .context("parsing GitHub matching-refs response")?;

    // Strip the `refs/tags/` prefix to leave the bare tag names.
    Ok(refs
        .into_iter()
        .filter_map(|r| r.name.strip_prefix("refs/tags/").map(String::from))
        .collect())
}

/// Build the GitHub release URL for the toolchain tarball.
///
/// With an explicit `version` the URL points at that exact release. Otherwise we
/// list the `zisk-<TOOLCHAIN_MAJOR>.*` tags via the matching-refs API and pick the
/// highest one (rather than `releases/latest`, which could jump to a newer major),
/// assuming a release exists under the same name.
pub(crate) async fn get_toolchain_download_url(
    client: &Client,
    target: &String,
    version: &Option<String>,
) -> Result<String> {
    let tag = if let Some(version) = version {
        version.clone()
    } else {
        let tags = fetch_matching_tags(client, TOOLCHAIN_MAJOR).await?;
        select_latest_tag(&tags, TOOLCHAIN_MAJOR).with_context(|| {
            format!("no `zisk-{TOOLCHAIN_MAJOR}.x.y` toolchain tag found for 0xPolygonHermez/rust")
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
    // Authenticate the download too, so a private fork's release assets are reachable.
    add_github_auth(&mut headers);
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
