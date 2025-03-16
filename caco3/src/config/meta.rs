use std::fmt::{Display, Formatter};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use toml::Value;

/// A new type struct of `toml::Value` to simplify parsing untyped configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct MetaConfig(Value);

const PATH_SEP: char = '.';

impl Display for MetaConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl MetaConfig {
    /// Get inner `&toml::Value`.
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// Get configuration of given dot separated path as `&toml::Value`.
    ///
    /// Examples
    ///
    /// ```rust
    /// use toml::toml;
    /// use caco3::config::MetaConfig;
    ///
    /// let inner = toml::Value::try_from(toml! {
    ///     [foo.bar]
    ///     baz = "hello"
    /// }).unwrap();
    /// let config = MetaConfig::from(inner);
    /// let expected = toml::Value::try_from(toml! {
    ///     baz = "hello"
    /// }).unwrap();
    /// assert_eq!(config.get("foo.bar"), &expected);
    /// ```
    pub fn get(&self, path: &str) -> &Value {
        let mut target = &self.0;
        for component in path.split(PATH_SEP) {
            target = &target[component];
        }
        target
    }
}

impl From<Value> for MetaConfig {
    fn from(val: Value) -> Self {
        Self(val)
    }
}

pub trait MetaConfigGetter {
    fn as_bool(&self, path: &str) -> Option<bool>;
    fn as_f64(&self, path: &str) -> Option<f64>;
    fn as_i64(&self, path: &str) -> Option<i64>;
    fn as_str(&self, path: &str) -> Option<&str>;
    fn to_offset_datetime(&self, path: &str) -> Option<OffsetDateTime>;
    fn to_instance<T: DeserializeOwned>(&self, path: &str) -> Option<T>;
}

impl MetaConfigGetter for MetaConfig {
    fn as_bool(&self, path: &str) -> Option<bool> {
        self.get(path).as_bool()
    }

    fn as_f64(&self, path: &str) -> Option<f64> {
        self.get(path).as_float()
    }

    fn as_i64(&self, path: &str) -> Option<i64> {
        self.get(path).as_integer()
    }

    fn as_str(&self, path: &str) -> Option<&str> {
        self.get(path).as_str()
    }

    fn to_offset_datetime(&self, path: &str) -> Option<OffsetDateTime> {
        let rfc3339 = self.get(path).as_datetime()?.to_string();
        OffsetDateTime::parse(&rfc3339, &time::format_description::well_known::Rfc3339).ok()
    }

    fn to_instance<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        self.get(path).clone().try_into().ok()
    }
}

macro_rules! impl_meta_config_getter_for_option {
    ($option:ty) => {
        impl MetaConfigGetter for $option {
            fn as_bool(&self, path: &str) -> Option<bool> {
                <MetaConfig as MetaConfigGetter>::as_bool(self.as_ref()?, path)
            }

            fn as_f64(&self, path: &str) -> Option<f64> {
                <MetaConfig as MetaConfigGetter>::as_f64(self.as_ref()?, path)
            }

            fn as_i64(&self, path: &str) -> Option<i64> {
                <MetaConfig as MetaConfigGetter>::as_i64(self.as_ref()?, path)
            }

            fn as_str(&self, path: &str) -> Option<&str> {
                <MetaConfig as MetaConfigGetter>::as_str(self.as_ref()?, path)
            }

            fn to_offset_datetime(&self, path: &str) -> Option<OffsetDateTime> {
                <MetaConfig as MetaConfigGetter>::to_offset_datetime(self.as_ref()?, path)
            }

            fn to_instance<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
                <MetaConfig as MetaConfigGetter>::to_instance(self.as_ref()?, path)
            }
        }
    };
}

impl_meta_config_getter_for_option!(Option<MetaConfig>);
impl_meta_config_getter_for_option!(Option<&MetaConfig>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Foo {
            bar: String,
        }
        let toml_content = r##"
            [test]
            bool = true
            str = "hello"

            [foo]
            bar = "bar"

            [time]
            offset_date_time = 1979-05-27T07:32:00Z
            thai_offset_date_time = 2022-09-06T09:43:22.123456789+07:00
            date_time = 1979-05-27T07:32:00
            date = 1979-05-27
            time = 1979-05-27T07:32:00
        "##;
        let config = MetaConfig(toml::from_str(toml_content).unwrap());
        assert_eq!(config.as_bool("test.bool"), Some(true));
        assert_eq!(Some(config.clone()).as_bool("test.bool"), Some(true));
        assert_eq!(Some(&config).as_bool("test.bool"), Some(true));

        assert_eq!(config.as_bool("test.bool"), Some(true));
        assert_eq!(config.as_str("test.str"), Some("hello"));
        assert_eq!(
            config.to_instance::<Foo>("foo"),
            Some(Foo {
                bar: "bar".to_string()
            })
        );

        assert_eq!(
            config.to_offset_datetime("time.offset_date_time"),
            Some(time::macros::datetime!(1979-05-27 07:32:00 +00:00))
        );
        assert_eq!(
            config.to_offset_datetime("time.thai_offset_date_time"),
            Some(time::macros::datetime!(2022-09-06 09:43:22.123456789 +07:00))
        );
        // unsupported conversion to OffsetDateTime
        assert_eq!(config.to_offset_datetime("time.date_time"), None);
        assert_eq!(config.to_offset_datetime("time.date"), None);
        assert_eq!(config.to_offset_datetime("time.time"), None);
    }
}
