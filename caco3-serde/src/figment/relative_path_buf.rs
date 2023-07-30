use serde::{Deserialize, Deserializer, Serialize, Serializer};

use private::Serde;

pub fn serialize<T, S>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    Serde<T>: Serialize,
{
    Serde::new_ref(val).serialize(serializer)
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    Serde<T>: Deserialize<'de>,
{
    Serde::deserialize(deserializer).map(Serde::into_inner)
}

mod private {
    use core::fmt;
    use std::path::PathBuf;

    use bytemuck::TransparentWrapper;
    use figment::value::magic::{Either, RelativePathBuf};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[repr(transparent)]
    #[derive(bytemuck::TransparentWrapper)]
    pub struct Serde<T>(T);

    impl<T> Serde<T> {
        pub(super) fn into_inner(self) -> T {
            self.0
        }

        pub(super) fn new_ref(inner_ref: &T) -> &Self {
            Self::wrap_ref(inner_ref)
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

    /// Human readable of `RelativePathBuf`
    ///
    /// Default representation of `RelativePathBuf` is for machine not human.
    /// This struct help serializing it in user friendly format.
    #[derive(Debug, Deserialize, Serialize)]
    struct ReadablePath {
        path: PathBuf,
    }

    impl Serialize for Serde<RelativePathBuf> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let Serde(path) = self;
            let readable_path = ReadablePath {
                path: path.relative(),
            };
            readable_path.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Serde<RelativePathBuf> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let path = match <Either<RelativePathBuf, ReadablePath>>::deserialize(deserializer)? {
                Either::Left(path) => path,
                Either::Right(ReadablePath { path }) => RelativePathBuf::from(path),
            };
            Ok(Serde(path))
        }
    }

    impl Serialize for Serde<Option<RelativePathBuf>> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match &self.0 {
                Some(path) => {
                    let serde_ref = Serde::new_ref(path);
                    serializer.serialize_some(serde_ref)
                }
                None => serializer.serialize_none(),
            }
        }
    }

    impl<'de> Deserialize<'de> for Serde<Option<RelativePathBuf>> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            match <Option<Serde<RelativePathBuf>>>::deserialize(deserializer)? {
                Some(Serde(val)) => Ok(Serde(Some(val))),
                None => Ok(Serde(None)),
            }
        }
    }

    macro_rules! impl_serialize_ref {
        (@deref $expr:expr, $lt:lifetime) => {
            * $expr
        };
        (@deref $expr:expr, $lt0:lifetime, $($lt:lifetime),+) => {
            * impl_serialize_ref!(@deref $expr, $($lt),+)
        };
        ($ty:ty, <$($lt:lifetime),+>) => {
            impl <$($lt),+> Serialize for Serde<$(&$lt)+ $ty> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    let inner_ref: &$ty = &impl_serialize_ref!(@deref self.0, $($lt),+);
                    let serde_ref: &Serde<$ty> = Serde::new_ref(inner_ref);
                    serde_ref.serialize(serializer)
                }
            }
        };
    }

    impl_serialize_ref!(RelativePathBuf, <'a>);
    impl_serialize_ref!(RelativePathBuf, <'a, 'b>);
    impl_serialize_ref!(Option<RelativePathBuf>, <'a>);
    impl_serialize_ref!(Option<RelativePathBuf>, <'a, 'b>);

    #[cfg(test)]
    mod tests {
        use std::path::PathBuf;

        use super::*;

        #[test]
        fn test_new_borrowed_safety() {
            let path = RelativePathBuf::from(PathBuf::from("/dev/null"));
            let _serde = Serde::new_ref(&path);
        }
    }
}
