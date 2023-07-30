use bytemuck::TransparentWrapper;
use serde::{Serialize, Serializer};

use private::Serde;

/// Serialize `byte_unit::Byte` using `byte.get_appropriate_unit(true)`.
pub fn serialize<T, S>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    Serde<T>: Serialize,
{
    Serde::wrap_ref(val).serialize(serializer)
}

mod private {
    use core::fmt;

    use byte_unit::Byte;
    use bytemuck::TransparentWrapper;
    use serde::{Serialize, Serializer};

    #[repr(transparent)]
    #[derive(bytemuck::TransparentWrapper)]
    pub struct Serde<T>(T);

    impl<T> Serde<T> {
        #[allow(dead_code)]
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

    impl Serialize for Serde<Byte> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let byte = self.0.get_appropriate_unit(true);
            Serialize::serialize(&byte, serializer)
        }
    }

    impl Serialize for Serde<Option<Byte>> {
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

    impl_serialize_ref!(Byte, <'a>);
    impl_serialize_ref!(Byte, <'a, 'b>);
    impl_serialize_ref!(Option<Byte>, <'a>);
    impl_serialize_ref!(Option<Byte>, <'a, 'b>);

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_new_borrowed_safety() {
            let byte = Byte::from_bytes(10);
            let _serde = Serde::new_ref(&byte);
        }

        #[test]
        fn test_serialize() {
            #[derive(Serialize)]
            struct BinaryByte(#[serde(serialize_with = "super::super::serialize")] Byte);
            let byte = BinaryByte(Byte::from_bytes(2 * 1024));
            let actual = serde_json::to_string(&byte).unwrap();
            assert_eq!(actual, r#""2.00 KiB""#);

            #[derive(Serialize)]
            struct BinaryOptionByte(
                #[serde(serialize_with = "super::super::serialize")] Option<Byte>,
            );
            let byte = BinaryOptionByte(None);
            let actual = serde_json::to_string(&byte).unwrap();
            assert_eq!(actual, r#"null"#);

            let byte = BinaryOptionByte(Some(Byte::from_bytes(2 * 1024)));
            let actual = serde_json::to_string(&byte).unwrap();
            assert_eq!(actual, r#""2.00 KiB""#);
        }
    }
}
