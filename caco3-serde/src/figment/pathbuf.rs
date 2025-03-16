use serde::{Deserialize, Deserializer};

use private::Serde;

pub fn deserialize_relative<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    Serde<T>: Deserialize<'de>,
{
    Serde::deserialize(deserializer).map(Serde::into_inner)
}

mod private {
    use core::fmt;
    use std::path::PathBuf;

    use figment::value::magic::RelativePathBuf;
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

    impl<'de> Deserialize<'de> for Serde<PathBuf> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let path = RelativePathBuf::deserialize(deserializer)?;
            Ok(Serde(path.relative()))
        }
    }

    impl<'de> Deserialize<'de> for Serde<Option<PathBuf>> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            match <Option<RelativePathBuf>>::deserialize(deserializer)? {
                Some(val) => Ok(Serde(Some(val.relative()))),
                None => Ok(Serde(None)),
            }
        }
    }
}
