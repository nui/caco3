use nix::pty::PtyMaster;
use std::ffi::c_int;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::path::PathBuf;

/// Resize a pseudo terminal using an ioctl.
pub fn resize_pty(file: BorrowedFd<'_>, width: u32, height: u32) -> io::Result<()> {
    unsafe {
        let winsz = libc::winsize {
            ws_col: width.try_into().unwrap_or(u16::MAX),
            ws_row: height.try_into().unwrap_or(u16::MAX),
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        #[allow(clippy::useless_conversion)] // Not useless on all platforms.
        check_return(libc::ioctl(
            file.as_raw_fd(),
            libc::TIOCSWINSZ.into(),
            &winsz,
        ))?;
        Ok(())
    }
}

/// Set the controlling terminal of the process group.
pub fn set_controlling_terminal_to_stdin() -> io::Result<()> {
    unsafe {
        #[allow(clippy::useless_conversion)] // Not useless on all platforms.
        check_return(libc::ioctl(0, libc::TIOCSCTTY.into(), 0))?;
        Ok(())
    }
}

/// Check the return value of a libc function that returns a `c_int`.
fn check_return(value: c_int) -> io::Result<c_int> {
    if value >= 0 {
        Ok(value)
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Create a new process group of which the calling process will be the session leader.
pub fn create_process_group() -> io::Result<()> {
    let _sid = nix::unistd::setsid()?;
    Ok(())
}

#[cfg(target_os = "linux")]
/// Get the path of the child terminal device.
pub fn get_child_terminal_path(pty_master: &PtyMaster) -> io::Result<PathBuf> {
    nix::pty::ptsname_r(pty_master)
        .map(PathBuf::from)
        .map_err(io::Error::from)
}

#[cfg(target_os = "macos")]
/// Get the path of the child terminal device.
pub fn get_child_terminal_path(pty_master: &PtyMaster) -> io::Result<PathBuf> {
    let slave_name = unsafe { nix::pty::ptsname(pty_master) };
    slave_name.map(PathBuf::from).map_err(io::Error::from)
}
