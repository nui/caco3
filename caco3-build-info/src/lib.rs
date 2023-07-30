#![deny(rust_2018_idioms)]

use std::process::Command;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use git2::{Repository, StatusOptions, StatusShow};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct BuildInfo {
    pub build_profile: String,
    pub build_target: String,
    pub epoch_seconds: u64,
    pub git: Option<Git>,
    pub rustc_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct Git {
    commit_id: CommitId,
    dirty: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct CommitId(String);

impl BuildInfo {
    pub fn get() -> Result<Self> {
        let build_target = std::env::var("TARGET")?;
        let build_profile = std::env::var("PROFILE")?;
        let epoch_seconds = get_epoch_seconds()?;
        let git = Git::get().ok();
        let rustc_version = get_rustc_version()?;
        Ok(Self {
            build_target,
            build_profile,
            epoch_seconds,
            git,
            rustc_version,
        })
    }
}

impl Git {
    pub fn get() -> Result<Self> {
        let repo = Repository::open_from_env()?;
        let commit_id = get_commit_id(&repo)?;
        let dirty = get_dirty(&repo)?;
        Ok(Git { commit_id, dirty })
    }

    /// Return a string which represent commit id and dirty status
    pub fn to_commit_and_dirty(&self) -> String {
        let mut output = String::with_capacity(self.commit_id.as_str().len() + 1);
        output.push_str(self.commit_id.as_str());
        if self.dirty {
            output.push('*');
        }
        output
    }

    pub fn from_commit_id_and_dirty(id: &str, dirty: bool) -> Option<Self> {
        is_valid_id(id).then(|| Self {
            commit_id: CommitId(id.to_string()),
            dirty,
        })
    }

    pub fn commit_id(&self) -> &CommitId {
        &self.commit_id
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }
}

impl CommitId {
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
}

impl FromStr for CommitId {
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
    let output = Command::new("rustc")
        .arg("--version")
        .output().context("get rustc version")?;
    if !output.status.success() {
        bail!("Failed to get rustc version");
    }
    let version = String::from_utf8(output.stdout).context("version output to String")?;
    Ok(version)
}

fn get_commit_id(repo: &Repository) -> Result<CommitId> {
    let revspec = repo.revparse("HEAD")?;
    let id = revspec
        .from()
        .context("HEAD commit should have 'from' range")?
        .id();
    Ok(CommitId(id.to_string()))
}

fn get_dirty(repo: &Repository) -> Result<bool> {
    let mut options = StatusOptions::new();
    options
        .exclude_submodules(true)
        .include_untracked(false)
        .show(StatusShow::IndexAndWorkdir);
    let statuses = repo.statuses(Some(&mut options))?;
    let clean = statuses.is_empty();
    Ok(!clean)
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
    fn git() {
        let commit_id = "12345678";
        let clean_git = Git {
            commit_id: CommitId(commit_id.to_string()),
            dirty: false,
        };
        let actual = clean_git.to_commit_and_dirty();
        assert_eq!(actual, commit_id);

        let dirty_git = Git {
            commit_id: CommitId(commit_id.to_string()),
            dirty: true,
        };
        let actual = dirty_git.to_commit_and_dirty();
        assert_eq!(actual, format!("{commit_id}*"));
    }

    #[test]
    fn git_from_commit_id() {
        assert!(Git::from_commit_id_and_dirty("123456", false).is_none());
        assert!(Git::from_commit_id_and_dirty("1234567", false).is_some());
        assert!(Git::from_commit_id_and_dirty("1234567-", false).is_none());
    }
}