use std::env;
use std::fmt::Display;
use std::fmt::Write;
use std::os::unix::prelude::CommandExt;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Jemalloc configuration.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Jemalloc {
    #[serde(default)]
    pub background_thread: bool,
    #[serde(default)]
    pub max_background_threads: Option<u32>,
    #[serde(default)]
    pub number_of_arenas: Option<u32>,
    #[serde(default)]
    pub extra_conf: Option<String>
}

pub const POSSIBLE_MALLOC_CONF_ENVIRONMENT_VARIABLES: &[&str] =
    &["MALLOC_CONF", "_RJEM_MALLOC_CONF"];

/// Return `true` if jemalloc background managed threads is supported.
pub const fn is_background_thread_supported() -> bool {
    // See https://github.com/tikv/jemallocator/blob/main/jemalloc-sys/src/env.rs
    if cfg!(target_env = "musl") {
        return false;
    }
    // Background thread on MacOS is not supported.
    // See https://github.com/jemalloc/jemalloc/issues/843
    if cfg!(target_os = "macos") {
        return false;
    }
    true
}

/// Re-execute current process to apply jemalloc configuration.
pub fn apply_config(config: &Jemalloc, f: impl FnOnce(&str)) -> ! {
    // Some configuration of jemalloc need to be configured before main program is started.
    // But at this point, main program has been started, how do we solve this?
    //
    // We replace current process with itself but with properly jemalloc configuration.
    let malloc_conf = config.to_config();

    let mut args = env::args_os();
    let program = args.next().expect("Process name");
    let mut cmd = Command::new(program);
    cmd.args(args);
    for name in POSSIBLE_MALLOC_CONF_ENVIRONMENT_VARIABLES {
        cmd.env(name, &malloc_conf);
    }
    f(&malloc_conf);
    let err = cmd.exec();
    panic!("jemalloc: exec error: {:?}", err);
}

/// Returns `true` if jemalloc is configured.
pub fn is_configured() -> bool {
    POSSIBLE_MALLOC_CONF_ENVIRONMENT_VARIABLES
        .iter()
        .any(|name| env::var_os(name).is_some())
}

impl Jemalloc {
    pub fn to_config(&self) -> String {
        let mut config = String::with_capacity(64);
        // Abort program if invalid jemalloc configurations are found.
        config.push_str("abort_conf:true");

        let mut write_config = |key: &str, value: &dyn Display| {
            write!(&mut config, ",{}:{}", key, value)
                .expect("a Display implementation returned an error unexpectedly");
        };
        if self.background_thread {
            // Do nothing, this is intended.
            // background thread should be enabled at runtime to avoid deadlock.
        }
        if let Some(v) = self.max_background_threads {
            write_config("max_background_threads", &v);
        }
        if let Some(v) = self.number_of_arenas {
            write_config("narenas", &v);
        }
        if let Some(extra_conf) = self.extra_conf.as_deref() {
            config.push(',');
            config.push_str(extra_conf);
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jemalloc_to_config() {
        let val = Jemalloc {
            background_thread: false,
            max_background_threads: None,
            number_of_arenas: None,
            extra_conf: None,
        };
        assert_eq!(val.to_config(), "abort_conf:true");

        let val = Jemalloc {
            background_thread: false,
            max_background_threads: None,
            number_of_arenas: None,
            extra_conf: Some("tcache:false".to_owned()),
        };
        assert_eq!(val.to_config(), "abort_conf:true,tcache:false");

        let val = Jemalloc {
            background_thread: false,
            max_background_threads: None,
            number_of_arenas: Some(16),
            extra_conf: None,
        };
        assert_eq!(val.to_config(), "abort_conf:true,narenas:16");

        let val = Jemalloc {
            background_thread: true,
            max_background_threads: None,
            number_of_arenas: None,
            extra_conf: None,
        };
        assert_eq!(val.to_config(), "abort_conf:true");

        let val = Jemalloc {
            background_thread: false,
            max_background_threads: Some(4),
            number_of_arenas: None,
            extra_conf: None,
        };
        assert_eq!(val.to_config(), "abort_conf:true,max_background_threads:4");

        let val = Jemalloc {
            background_thread: true,
            max_background_threads: Some(8),
            number_of_arenas: Some(64),
            extra_conf: None,
        };
        assert_eq!(
            val.to_config(),
            "abort_conf:true,max_background_threads:8,narenas:64"
        );
    }
}
