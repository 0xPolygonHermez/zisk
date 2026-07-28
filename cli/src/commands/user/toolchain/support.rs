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

/// Run a [`Command`] inheriting stdio, turning a spawn/IO failure **or a non-zero
/// exit status** into an error.
pub(crate) trait CommandExecutor {
    fn run(&mut self) -> Result<()>;
}

impl CommandExecutor for Command {
    fn run(&mut self) -> Result<()> {
        let status = self
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .status()
            .with_context(|| format!("while executing `{self:?}`"))?;
        // A non-zero exit must be an error: otherwise a failed step (e.g. `x.py
        // build` aborting because the LLVM patch did not apply) is silently
        // treated as success and we go on to package/publish a broken toolchain.
        if !status.success() {
            anyhow::bail!("command exited with {status}: `{self:?}`");
        }
        Ok(())
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
const TOOLCHAIN_MAJOR: u64 = 2;

/// Git URL of the ZisK Rust fork, used to list toolchain tags with `git ls-remote`
/// (git protocol, so no REST API rate limit).
const TOOLCHAIN_REPO_GIT_URL: &str = "https://github.com/0xPolygonHermez/rust.git";

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

/// List the `zisk-<major>.*` tag names of `0xPolygonHermez/rust` with
/// `git ls-remote` (primary path).
///
/// This uses the git smart-HTTP protocol against `github.com`, **not** the REST
/// API, so it is not subject to the 60 req/h unauthenticated REST rate limit. The
/// refspec pattern scopes the listing to this major's tags, keeping us clear of
/// the thousands of upstream Rust tags. We assume each tag has a release published
/// under the same name.
///
/// The literal `.` in the `zisk-<major>.*` glob enforces the boundary, so
/// `zisk-1.*` never matches `zisk-10.*` (nor the rolling `zisk-1-latest`).
///
/// Returns `Err` when `git` is not installed or the command fails, so the caller
/// can fall back to the REST API.
fn fetch_matching_tags_git(major: u64) -> Result<Vec<String>> {
    let pattern = format!("refs/tags/zisk-{major}.*");
    let output = Command::new("git")
        .args(["ls-remote", "--tags", "--refs", TOOLCHAIN_REPO_GIT_URL, &pattern])
        .output()
        .context("running `git ls-remote` to list toolchain tags")?;

    if !output.status.success() {
        anyhow::bail!(
            "`git ls-remote {TOOLCHAIN_REPO_GIT_URL}` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    // Each line is "<sha>\trefs/tags/<tag>"; keep the bare tag name.
    let stdout = String::from_utf8(output.stdout).context("`git ls-remote` output is not UTF-8")?;
    Ok(stdout
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .filter_map(|reference| reference.strip_prefix("refs/tags/"))
        .map(String::from)
        .collect())
}

/// List the `zisk-<major>.*` tag names via the GitHub matching-refs REST API
/// (fallback path, used only when `git` is unavailable).
///
/// This hits `api.github.com`, so it is subject to the unauthenticated 60 req/h
/// per-IP limit (raised to 5000 req/h when `GITHUB_TOKEN`/`GH_TOKEN` is set).
async fn fetch_matching_tags_rest(client: &Client, major: u64) -> Result<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct Ref {
        #[serde(rename = "ref")]
        name: String,
    }

    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("cargo-zisk"));
    headers.insert("Accept", HeaderValue::from_static("application/vnd.github+json"));
    add_github_auth(&mut headers);

    // `per_page=100` (the max) because matching-refs paginates at 30 by default:
    // with the default page we could miss newer `zisk-<major>.*` tags and pick a
    // non-latest toolchain. 100 covers any realistic number of releases per major.
    let url = format!(
        "https://api.github.com/repos/0xPolygonHermez/rust/git/matching-refs/tags/zisk-{major}.?per_page=100"
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

    Ok(refs
        .into_iter()
        .filter_map(|r| r.name.strip_prefix("refs/tags/").map(String::from))
        .collect())
}

/// Build the GitHub release URL for the toolchain tarball.
///
/// With an explicit `version` the URL points at that exact release. Otherwise we
/// list the `zisk-<TOOLCHAIN_MAJOR>.*` tags and pick the highest one (rather than
/// `releases/latest`, which could jump to a newer major), assuming a release
/// exists under the same name. Tags are listed via `git ls-remote` (no REST rate
/// limit); if `git` is unavailable we fall back to the REST API.
pub(crate) async fn get_toolchain_download_url(
    client: &Client,
    target: &str,
    version: &Option<String>,
) -> Result<String> {
    let tag = match version {
        Some(version) => version.clone(),
        None => {
            // Prefer `git ls-remote` (no REST rate limit). If git is missing or
            // fails, silently fall back to the REST API — not having git is a
            // valid setup and shouldn't surface an alarming message to the user.
            let tags = match fetch_matching_tags_git(TOOLCHAIN_MAJOR) {
                Ok(tags) => tags,
                Err(_) => fetch_matching_tags_rest(client, TOOLCHAIN_MAJOR).await?,
            };
            select_latest_tag(&tags, TOOLCHAIN_MAJOR).with_context(|| {
                format!(
                    "no `zisk-{TOOLCHAIN_MAJOR}.x.y` toolchain tag found for 0xPolygonHermez/rust"
                )
            })?
        }
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
        .or(Err(format!("Failed to GET from '{}'", url)))?;
    let total_size =
        res.content_length().ok_or(format!("Failed to get content length from '{}'", url))?;

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
        // The pinned-version branch neither runs git nor touches the network, so
        // any client works.
        let client = Client::new();
        let url = get_toolchain_download_url(
            &client,
            "x86_64-unknown-linux-gnu",
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
