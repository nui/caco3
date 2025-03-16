use std::path::{Path, PathBuf};
use std::{fs, io};

use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize, Debug)]
struct WorkspaceRootConfig {
    #[allow(dead_code)]
    workspace: Workspace,
}

#[derive(Deserialize, Debug)]
struct Workspace {
    #[allow(dead_code)]
    members: Vec<String>,
}

#[derive(Error, Debug)]
pub enum DiscoverWorkspaceError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("not found workspace directory")]
    NotFound,
}

/// Attempt to find an already-existing workspace at or above path.
///
/// Workspace is a directory which contain `Cargo.toml` file with workspace settings.
///
/// Example workspace file
/// ```toml
/// [workspace]
/// members = [
///     "common",
///     "api",
/// ]
/// ```
pub fn discover_workspace<P: AsRef<Path>>(path: P) -> Result<PathBuf, DiscoverWorkspaceError> {
    let mut dir = path.as_ref().to_path_buf();
    const CARGO_CONFIG_FILE: &str = "Cargo.toml";
    loop {
        let workspace_config = dir.join(CARGO_CONFIG_FILE);
        if workspace_config.exists() {
            let text = fs::read_to_string(&workspace_config)?;
            if toml::from_str::<WorkspaceRootConfig>(&text).is_ok() {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            return Err(DiscoverWorkspaceError::NotFound);
        }
    }
}
