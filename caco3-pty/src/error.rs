use std::error::Error;
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum AllocateError {
    Open(std::io::Error),
    Grant(std::io::Error),
    Unlock(std::io::Error),
    GetChildName(std::io::Error),
    OpenChild(std::io::Error),
}

#[derive(Debug)]
pub enum SpawnError {
    DuplicateStdio(std::io::Error),
    CreateSession(std::io::Error),
    SetControllingTerminal(std::io::Error),
    Spawn(std::io::Error),
    WrapAsyncFd(std::io::Error),
}

#[derive(Debug)]
pub struct ResizeError(pub std::io::Error);

impl Error for AllocateError {}
impl Error for SpawnError {}
impl Error for ResizeError {}

impl Display for AllocateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocateError::Open(err) => write!(f, "failed to open new pseudo terminal: {err}"),
            AllocateError::Grant(err) => write!(
                f,
                "failed to grant permissions on child terminal device: {err}"
            ),
            AllocateError::Unlock(err) => {
                write!(f, "failed to unlock child terminal device: {err}")
            }
            AllocateError::GetChildName(err) => {
                write!(f, "failed to get name of child terminal device: {err}")
            }
            AllocateError::OpenChild(err) => {
                write!(f, "failed to open child terminal device: {err}")
            }
        }
    }
}

impl Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpawnError::DuplicateStdio(err) => write!(
                f,
                "failed to duplicate file descriptor for standard I/O stream: {err}"
            ),
            SpawnError::CreateSession(err) => {
                write!(f, "failed to create new process group: {err}")
            }
            SpawnError::SetControllingTerminal(err) => write!(
                f,
                "failed to set controlling terminal for new process group: {err}"
            ),
            SpawnError::Spawn(err) => write!(f, "failed to spawn child process: {err}"),
            SpawnError::WrapAsyncFd(err) => write!(
                f,
                "failed to wrap pseudo terminal file descriptor for use with tokio: {err}"
            ),
        }
    }
}

impl Display for ResizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(e) = self;
        write!(f, "failed to resize terminal device: {e}")
    }
}
