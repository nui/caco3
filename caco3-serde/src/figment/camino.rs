use serde::{Deserialize, Deserializer};
use std::fmt;
use std::fmt::Formatter;
use std::path::PathBuf;

use private::Serde;

pub fn deserialize_relative<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    Serde<T>: Deserialize<'de>,
{
    Serde::deserialize(deserializer).map(Serde::into_inner)
}

mod private {
    use crate::figment::camino::NonUtf8PathError;
    use camino::Utf8PathBuf;
    use core::fmt;
    use figment::value::magic::RelativePathBuf;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer};

    pub struct Serde<T>(T);

    impl<T> Serde<T> {
        pub(super) fn into_inner(self) -> T {
            self.0
        }
    }

    impl<T> fmt::Debug for Serde<T>
    where
        T: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl<'de> Deserialize<'de> for Serde<Utf8PathBuf> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let path = RelativePathBuf::deserialize(deserializer)?;
            let relative_path = path.relative();
            let utf8path = Utf8PathBuf::from_path_buf(relative_path)
                .map_err(|path| Error::custom(NonUtf8PathError(path)))?;
            Ok(Serde(utf8path))
        }
    }

    impl<'de> Deserialize<'de> for Serde<Option<Utf8PathBuf>> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            match <Option<RelativePathBuf>>::deserialize(deserializer)? {
                Some(path) => {
                    let relative_path = path.relative();
                    let utf8path = Utf8PathBuf::from_path_buf(relative_path)
                        .map_err(|path| Error::custom(NonUtf8PathError(path)))?;
                    Ok(Serde(Some(utf8path)))
                }
                None => Ok(Serde(None)),
            }
        }
    }
}

struct NonUtf8PathError(PathBuf);

impl fmt::Display for NonUtf8PathError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} has invalid utf8 bytes", self.0.display())
    }
}
