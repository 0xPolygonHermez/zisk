use anyhow::Result;
use std::{fs, path::Path, process::Command};
use yansi::Paint;
use zisk_build::ZISK_VERSION_MESSAGE;

/// Branch of the `zisk_template` repo cloned by default. A blank, compilable
/// placeholder project whose crate names use the `template-` prefix.
const BLANK_TEMPLATE_BRANCH: &str = "feature/blank-template";

/// Crate-name prefix used throughout the template; rewritten to the project name.
const TEMPLATE_PREFIX: &str = "template-";

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Create a new project that runs inside ZisK
pub(crate) struct NewCmd {
    /// Name of the new project to create
    name: String,
}

impl NewCmd {
    pub(crate) fn run(&self) -> Result<()> {
        let root = Path::new(&self.name);
        let repo_url = "https://{}@github.com/0xPolygonHermez/zisk_template";
        // Create the root directory if it doesn't exist.
        if !root.exists() {
            fs::create_dir(&self.name)?;
        }

        // Branch to clone. Defaults to the blank template; `ZISK_TEMPLATE_BRANCH`
        // overrides it (an empty value falls back to the default).
        let branch = std::env::var("ZISK_TEMPLATE_BRANCH")
            .ok()
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| BLANK_TEMPLATE_BRANCH.to_string());

        // Clone the repository.
        let mut cmd = Command::new("git");
        cmd.args(Self::git_clone_args(
            repo_url,
            root.as_os_str().to_str().unwrap(),
            Some(branch.as_str()),
        ));

        let output = cmd.output().expect("failed to execute command");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("failed to clone repository: {}", stderr));
        }

        // Remove the .git directory.
        fs::remove_dir_all(root.join(".git"))?;

        // Rewrite the `template-` crate-name prefix to the new project name across
        // the copied sources, so the scaffolded project builds under its own name.
        let project_name = sanitize(&self.name);
        rewrite_template_prefix(root, &project_name)?;

        println!(
            "    \x1b[1m{}\x1b[0m {} ({})",
            Paint::green("Initialized"),
            self.name,
            std::fs::canonicalize(root).expect("failed to canonicalize").to_str().unwrap()
        );

        Ok(())
    }

    /// Assemble the `git clone` argument vector. Pure: the optional `--branch`
    /// is appended only for a non-empty branch, so the env-driven branch
    /// selection can be tested without invoking git.
    fn git_clone_args(repo_url: &str, dir: &str, branch: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "clone".to_string(),
            repo_url.to_string(),
            dir.to_string(),
            "--recurse-submodules".to_string(),
            "--depth=1".to_string(),
        ];
        if let Some(branch) = branch.filter(|b| !b.is_empty()) {
            args.push("--branch".to_string());
            args.push(branch.to_string());
        }
        args
    }
}

/// Normalise a raw project name into a valid Cargo crate-name fragment:
/// lowercase, keeping only ASCII alphanumerics, `-` and `_`; any other character becomes `-`.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

/// Rewrite the in-file occurrences of `template-` to `<name>-`.
fn rewrite_contents(contents: &str, name: &str) -> String {
    contents.replace(TEMPLATE_PREFIX, &format!("{name}-"))
}

/// Walk `dir` recursively, rewriting the `template-` prefix in every `.rs` or `.toml` file to `<name>-`.
fn rewrite_template_prefix(dir: &Path, name: &str) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rewrite_template_prefix(&path, name)?;
            continue;
        }
        if matches!(path.extension().and_then(|e| e.to_str()), Some("rs" | "toml")) {
            let contents = fs::read_to_string(&path)?;
            let rewritten = rewrite_contents(&contents, name);
            if rewritten != contents {
                fs::write(&path, rewritten)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{rewrite_contents, sanitize, NewCmd};

    const URL: &str = "https://{}@github.com/0xPolygonHermez/zisk_template";

    #[test]
    fn clone_args_without_branch() {
        let args = NewCmd::git_clone_args(URL, "myproj", None);
        assert_eq!(args, vec!["clone", URL, "myproj", "--recurse-submodules", "--depth=1"]);
        assert!(!args.iter().any(|a| a == "--branch"));
    }

    #[test]
    fn clone_args_empty_branch_is_ignored() {
        let args = NewCmd::git_clone_args(URL, "myproj", Some(""));
        assert!(!args.iter().any(|a| a == "--branch"));
    }

    #[test]
    fn clone_args_with_branch() {
        let args = NewCmd::git_clone_args(URL, "myproj", Some("dev"));
        assert!(args.windows(2).any(|w| w == ["--branch", "dev"]));
    }

    #[test]
    fn sanitize_lowercases_and_keeps_dashes() {
        assert_eq!(sanitize("My-Proof_1"), "my-proof_1");
    }

    #[test]
    fn sanitize_replaces_invalid_chars_with_dash() {
        assert_eq!(sanitize("my proof!"), "my-proof-");
    }

    #[test]
    fn rewrite_replaces_all_prefix_sites() {
        let src = r#"name = "template-guest"
load_program!("template-guest");
deps = ["template-host", "template-common"]"#;
        let out = rewrite_contents(src, "myproof");
        assert_eq!(
            out,
            r#"name = "myproof-guest"
load_program!("myproof-guest");
deps = ["myproof-host", "myproof-common"]"#
        );
    }

    #[test]
    fn rewrite_leaves_bare_word_template_untouched() {
        let src = "// This is the template project, edit template-guest below.";
        let out = rewrite_contents(src, "myproof");
        assert_eq!(out, "// This is the template project, edit myproof-guest below.");
    }
}
