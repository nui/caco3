//! Utility module to use time.rs OffsetDateTime in multi-threaded app on unix platform.

use std::io::ErrorKind;
use std::sync::OnceLock;

use time::{OffsetDateTime, UtcOffset};
use tz::TimeZone;
use tz::TzError;

use crate::config::is_truthy;

/// Get now in local timezone.
///
/// Please see [`local_utc_offset`] on how UtcOffset is determined.
pub fn local_now() -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    let unix_timestamp = now.unix_timestamp();
    let local_utc_offset = local_utc_offset_impl(Some(unix_timestamp));
    now.to_offset(local_utc_offset)
}

/// Get local UtcOffset.
///
/// We do timezone caching by default because it is quite costly operation.
/// The effect of caching is that when timezone database file is updated, we don't get new
/// value until process is restarted. This is usually fine for place where local timezone
/// doesn't use daylight saving time.
///
/// To disable timezone caching, set `CACO3_CACHE_TIMEZONE` environment variable to `false`.
pub fn local_utc_offset() -> UtcOffset {
    local_utc_offset_impl(None)
}

fn local_utc_offset_impl(unix_timestamp: Option<i64>) -> UtcOffset {
    static USE_CACHE_TIMEZONE: OnceLock<bool> = OnceLock::new();

    let use_cache_timezone = *USE_CACHE_TIMEZONE.get_or_init(|| {
        // Timezone is cached by default
        std::env::var("CACO3_CACHE_TIMEZONE").map_or(true, is_truthy)
    });

    if cfg!(unix) {
        // Unix system can't use UtcOffset detection because of following issue.
        // https://github.com/time-rs/time/issues/293
        get_unix_local_utc_offset(use_cache_timezone, unix_timestamp)
            .expect("couldn't determine UtcOffset for unix platform, Invalid /etc/localtime  or TZ is unset")
    } else {
        UtcOffset::current_local_offset().expect("Non-unix platform can get local offset")
    }
}

fn is_io_not_found(error: &TzError) -> bool {
    matches!(error, TzError::IoError(err) if err.kind() == ErrorKind::NotFound)
}

fn get_unix_timezone() -> Result<TimeZone, TzError> {
    if let Ok(tz_string) = std::env::var("TZ") {
        // We fallback if TZ is incorrect
        TimeZone::from_posix_tz(&tz_string).or_else(|_| TimeZone::local())
    } else {
        TimeZone::local()
    }
}

/// find local time using pure Rust implementation from tz-rs crate
fn get_unix_local_utc_offset(
    use_cache_timezone: bool,
    unix_timestamp: Option<i64>,
) -> Result<UtcOffset, TzError> {
    static TIMEZONE: OnceLock<TimeZone> = OnceLock::new();

    let mut non_cached_timezone = None;
    let timezone_result = if use_cache_timezone {
        // We don't care about timezone file contents change
        // The first call will cache timezone information.
        let tz = match TIMEZONE.get() {
            Some(val) => val,
            None => {
                let tz = get_unix_timezone()?;
                TIMEZONE.get_or_init(|| tz)
            }
        };
        Ok(tz)
    } else {
        // I believe this is also affected by https://github.com/time-rs/time/issues/293
        // Although the chance is very low on our system.
        get_unix_timezone().map(|v| &*non_cached_timezone.insert(v))
    };

    let timezone = match timezone_result {
        Ok(val) => val,
        Err(err) if is_io_not_found(&err) => {
            // Fallback to UTC
            return Ok(UtcOffset::UTC);
        }
        Err(err) => return Err(err),
    };

    let local_time_type = match unix_timestamp {
        Some(unix_timestamp) => timezone.find_local_time_type(unix_timestamp)?,
        None => timezone.find_current_local_time_type()?,
    };
    let seconds = local_time_type.ut_offset();
    Ok(UtcOffset::from_whole_seconds(seconds).expect("tz-rs returns valid utc offset seconds"))
}
