use std::time::{Duration, SystemTime, UNIX_EPOCH};

use time::UtcOffset;

#[cfg(feature = "local-offset")]
pub use local_time::{local_now, local_utc_offset};

pub mod human_duration;
#[cfg(feature = "local-offset")]
mod local_time;

/// Thailand utc offset (+07:00).
pub const THAILAND_UTC_OFFSET: UtcOffset = {
    match UtcOffset::from_hms(7, 0, 0) {
        Ok(val) => val,
        Err(_) => unreachable!()
    }
};

pub fn duration_since_unix_time(unix_time: u64) -> Option<Duration> {
    let now_epoch = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    now_epoch.checked_sub(Duration::from_secs(unix_time))
}