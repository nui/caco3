#![deny(rust_2018_idioms)]

use std::process::Command;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct BuildInfo {
    pub build_profile: String,
    pub build_target: String,
    pub epoch_seconds: u64,
    pub git_sha: Option<GitSha>,
    pub rustc_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct GitSha(String);

impl BuildInfo {
    pub fn from_build_script() -> Result<Self> {
        let build_target = std::env::var("TARGET")?;
        let build_profile = std::env::var("PROFILE")?;
        let epoch_seconds = get_epoch_seconds()?;
        let git_sha = GitSha::from_cmd();
        let rustc_version = get_rustc_version()?;
        Ok(Self {
            build_target,
            build_profile,
            epoch_seconds,
            git_sha,
            rustc_version,
        })
    }
}

impl GitSha {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn shorten(&self, max_length: usize) -> &str {
        let id = self.0.as_str();
        let index = id
            .char_indices()
            .nth(max_length)
            .map(|(index, _)| index)
            .unwrap_or(id.len());
        &id[..index]
    }

    pub fn from_cmd() -> Option<Self> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .ok()?;
        let mut stdout = String::from_utf8(output.stdout).ok()?;
        stdout.truncate(stdout.trim().len());
        if is_valid_id(&stdout) {
            Some(GitSha(stdout))
        } else {
            None
        }
    }
}

impl FromStr for GitSha {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if is_valid_id(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(())
        }
    }
}

fn is_valid_id(id: &str) -> bool {
    let len = id.len();
    const MIN_SHORT_COMMIT_ID_LEN: usize = 7;
    const BITS_PER_CHAR: usize = 4;
    // Git is using SHA1 but will move to SHA256 soon.
    const POSSIBLE_FULL_COMMIT_ID_LEN: usize = 256 / BITS_PER_CHAR;
    let valid_git_hash = (MIN_SHORT_COMMIT_ID_LEN..=POSSIBLE_FULL_COMMIT_ID_LEN).contains(&len)
        && id.chars().all(|c: char| c.is_ascii_hexdigit());
    valid_git_hash
}

fn get_rustc_version() -> Result<String> {
    let rustc = std::env::var("RUSTC_WRAPPER").ok();
    let rustc = rustc.as_deref().unwrap_or("rustc");
    let output = Command::new(rustc)
        .arg("--version")
        .output()
        .context("get rustc version")?;
    if !output.status.success() {
        bail!("Failed to get rustc version");
    }
    let version = core::str::from_utf8(&output.stdout)
        .context("version output to utf8")?
        .trim();
    Ok(version.into())
}

fn get_epoch_seconds() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[macro_export]
macro_rules! rustc_env {
    ($name:expr, $value:expr) => {
        println!("cargo:rustc-env={}={}", $name, $value);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_valid_id() {
        assert!(is_valid_id("1460ba33e88a6caff86948da489be527fa442a9a"));
    }
}
