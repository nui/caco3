use nix::fcntl::OFlag;
use nix::pty::PtyMaster;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::AsFd;
use std::task::{ready, Poll};
use tokio::io::unix::AsyncFd;
use tokio::process::Child;

use crate::sys::get_child_terminal_path;
use crate::{sys, AllocateError, ResizeError, SpawnError};

pub struct PtyPair {
    pty_master: PtyMaster,
    child_pty: File,
}

impl PtyPair {
    /// Allocate a new pseudo terminal with file descriptors for the parent and child end of the terminal.
    fn new() -> Result<Self, AllocateError> {
        let pty_master = nix::pty::posix_openpt(
            OFlag::O_RDWR | OFlag::O_NOCTTY | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC,
        )
        .map_err(io::Error::from)
        .map_err(AllocateError::Open)?;
        nix::pty::grantpt(&pty_master)
            .map_err(io::Error::from)
            .map_err(AllocateError::Grant)?;
        nix::pty::unlockpt(&pty_master)
            .map_err(io::Error::from)
            .map_err(AllocateError::Unlock)?;
        let child_pty_path =
            get_child_terminal_path(&pty_master).map_err(AllocateError::GetChildName)?;
        let child_pty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(child_pty_path)
            .map_err(AllocateError::OpenChild)?;
        Ok(Self {
            pty_master,
            child_pty,
        })
    }

    /// Spawn a child process as the session leader of a new process group with the pseudo terminal as controlling terminal.
    ///
    /// Also returns the parent side of the pseudo terminal as [`PseudoTerminal`] object.
    pub async fn spawn(
        self,
        mut command: tokio::process::Command,
    ) -> Result<(PseudoTerminal, Child), SpawnError> {
        let Self {
            pty_master,
            child_pty: child_tty_file,
        } = self;
        let stdin = child_tty_file;
        let stdout = stdin.try_clone().map_err(SpawnError::DuplicateStdio)?;
        let stderr = stdin.try_clone().map_err(SpawnError::DuplicateStdio)?;
        command.stdin(stdin);
        command.stdout(stdout);
        command.stderr(stderr);

        unsafe {
            command.pre_exec(move || {
                sys::create_process_group()
                    .map_err(SpawnError::CreateSession)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                sys::set_controlling_terminal_to_stdin()
                    .map_err(SpawnError::SetControllingTerminal)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                Ok(())
            });
        };
        let child = command.spawn().map_err(SpawnError::Spawn)?;
        let pty = PseudoTerminal::new(pty_master)?;
        Ok((pty, child))
    }
}

pub struct PseudoTerminal {
    inner: AsyncFd<PtyMaster>,
}

impl PseudoTerminal {
    /// Allocate a new pseudo terminal.
    pub fn allocate() -> Result<PtyPair, AllocateError> {
        PtyPair::new()
    }

    fn new(pty_master: PtyMaster) -> Result<Self, SpawnError> {
        Ok(Self {
            inner: AsyncFd::new(pty_master).map_err(SpawnError::WrapAsyncFd)?,
        })
    }

    /// Resize the pseudo-terminal.
    ///
    /// Should be called when the terminal emulator changes size.
    pub fn resize(&self, width: u32, height: u32) -> Result<(), ResizeError> {
        sys::resize_pty(self.inner.as_fd(), width, height).map_err(ResizeError)
    }
}

impl tokio::io::AsyncRead for PseudoTerminal {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        poll_read_impl(&self.inner, cx, buf)
    }
}

impl tokio::io::AsyncRead for &PseudoTerminal {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        poll_read_impl(&self.inner, cx, buf)
    }
}

fn poll_read_impl(
    fd: &AsyncFd<PtyMaster>,
    cx: &mut std::task::Context<'_>,
    buf: &mut tokio::io::ReadBuf<'_>,
) -> Poll<io::Result<()>> {
    loop {
        let mut guard = ready!(fd.poll_read_ready(cx))?;

        let unfilled = buf.initialize_unfilled();
        match guard.try_io(|inner| inner.get_ref().read(unfilled)) {
            Ok(Ok(len)) => {
                buf.advance(len);
                return Poll::Ready(Ok(()));
            }
            Ok(Err(err)) => return Poll::Ready(Err(err)),
            Err(_would_block) => continue,
        }
    }
}

impl tokio::io::AsyncWrite for PseudoTerminal {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        poll_write_impl(&self.inner, cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl tokio::io::AsyncWrite for &PseudoTerminal {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        poll_write_impl(&self.inner, cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

fn poll_write_impl(
    fd: &AsyncFd<PtyMaster>,
    cx: &mut std::task::Context<'_>,
    buf: &[u8],
) -> Poll<Result<usize, io::Error>> {
    loop {
        let mut guard = ready!(fd.poll_write_ready(cx))?;
        match guard.try_io(|inner| inner.get_ref().write(buf)) {
            Ok(result) => return Poll::Ready(result),
            Err(_would_block) => continue,
        }
    }
}
