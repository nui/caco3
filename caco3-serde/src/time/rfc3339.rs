//! Helper module for serializing/deserializing datetime using rfc3339 standard
//!
//! Examples
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use time::macros::datetime;
//! use time::OffsetDateTime;
//!
//! let datetime = datetime!(2022-01-01 01:23:45.123456789+07:00);
//!
//! #[derive(Serialize, Deserialize)]
//! #[serde(transparent)]
//! struct Millisecond(#[serde(with = "caco3_serde::time::rfc3339::millisecond")] OffsetDateTime);
//!
//! let rfc3339_millisecond = serde_json::to_string(&Millisecond(datetime)).unwrap();
//! assert_eq!(rfc3339_millisecond, r#""2022-01-01T01:23:45.123+07:00""#);
//! let actual = serde_json::from_str::<Millisecond>(&rfc3339_millisecond).unwrap().0;
//! assert_eq!(actual, datetime!(2022-01-01 01:23:45.123+07:00));
//!
//! #[derive(Serialize, Deserialize)]
//! #[serde(transparent)]
//! struct Second(#[serde(with = "caco3_serde::time::rfc3339::second")] OffsetDateTime);
//!
//! let rfc3339_second = serde_json::to_string(&Second(datetime)).unwrap();
//! assert_eq!(rfc3339_second, r#""2022-01-01T01:23:45+07:00""#);
//! let actual = serde_json::from_str::<Second>(&rfc3339_second).unwrap().0;
//! assert_eq!(actual, datetime!(2022-01-01 01:23:45+07:00));
//! ```

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

macro_rules! declare_serde_module {
    ($unit:ty) => {
        use serde::de::DeserializeOwned;
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        use super::private::*;

        pub fn serialize<T, S>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
        where
            T: Copy,
            S: Serializer,
            Serde<T, $unit>: Serialize,
        {
            <Serde<_, $unit>>::new(*val).serialize(serializer)
        }

        pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
        where
            D: Deserializer<'de>,
            Serde<T, $unit>: DeserializeOwned,
        {
            Serde::deserialize(deserializer).map(Serde::into_time)
        }
    };
}

pub mod millisecond {
    declare_serde_module!(MillisecondUnit);
}
pub mod second {
    declare_serde_module!(SecondUnit);
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Millisecond(#[serde(with = "millisecond")] pub OffsetDateTime);

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Second(#[serde(with = "second")] pub OffsetDateTime);

mod private {
    use std::marker::PhantomData;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::OffsetDateTime;

    pub struct MillisecondUnit;
    pub struct SecondUnit;

    /// Generalizing serialization/deserialization over `OffsetDateTime`
    pub struct Serde<T, U> {
        time: T,
        unit: PhantomData<U>,
    }

    impl<T, U> Serde<T, U> {
        pub(super) fn into_time(self) -> T {
            self.time
        }
    }

    macro_rules! impl_serde {
        ($ty:ty, $unit:ty, $rounder:path) => {
            impl<T> Serde<T, $unit> {
                pub(super) fn new(time: T) -> Self {
                    Self {
                        time,
                        unit: PhantomData,
                    }
                }
            }

            impl Serialize for Serde<$ty, $unit> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    let datetime = $rounder(self.time);
                    ::time::serde::rfc3339::serialize(&datetime, serializer)
                }
            }

            impl<'de> Deserialize<'de> for Serde<$ty, $unit> {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    let datetime: $ty = ::time::serde::rfc3339::deserialize(deserializer)?;
                    Ok(<Serde<_, $unit>>::new($rounder(datetime)))
                }
            }

            impl Serialize for Serde<Option<$ty>, $unit> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    match self.time {
                        Some(val) => serializer.serialize_some(&<Serde<_, $unit>>::new(val)),
                        None => serializer.serialize_none(),
                    }
                }
            }

            impl<'de> Deserialize<'de> for Serde<Option<$ty>, $unit> {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    match <Option<Serde<$ty, $unit>>>::deserialize(deserializer)? {
                        Some(Serde { time, .. }) => Ok(<Serde<_, $unit>>::new(Some(time))),
                        None => Ok(<Serde<_, $unit>>::new(None)),
                    }
                }
            }
        };
    }

    impl_serde!(OffsetDateTime, MillisecondUnit, floor_to_millisecond);
    impl_serde!(OffsetDateTime, SecondUnit, floor_to_second);

    // n.b. `$ty` must implement Copy
    macro_rules! impl_serialize_ref {
        (@deref $expr:expr, $lt:lifetime) => {
            * $expr
        };
        (@deref $expr:expr, $lt0:lifetime, $($lt:lifetime),+) => {
            * impl_serialize_ref!(@deref $expr, $($lt),+)
        };
        ($unit:ty, $ty:ty, <$($lt:lifetime),+>) => {
            impl <$($lt),+> Serialize for Serde<$(&$lt)+ $ty, $unit> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    let time: $ty = impl_serialize_ref!(@deref self.time, $($lt),+);
                    let serde = <Serde<_, $unit>>::new(time);
                    serde.serialize(serializer)
                }
            }
        };
    }

    impl_serialize_ref!(MillisecondUnit, OffsetDateTime, <'a>);
    impl_serialize_ref!(MillisecondUnit, OffsetDateTime, <'a, 'b>);
    impl_serialize_ref!(MillisecondUnit, Option<OffsetDateTime>, <'a>);
    impl_serialize_ref!(MillisecondUnit, Option<OffsetDateTime>, <'a, 'b>);

    impl_serialize_ref!(SecondUnit, OffsetDateTime, <'a>);
    impl_serialize_ref!(SecondUnit, OffsetDateTime, <'a, 'b>);
    impl_serialize_ref!(SecondUnit, Option<OffsetDateTime>, <'a>);
    impl_serialize_ref!(SecondUnit, Option<OffsetDateTime>, <'a, 'b>);

    fn floor_to_millisecond(datetime: OffsetDateTime) -> OffsetDateTime {
        datetime
            .replace_millisecond(datetime.millisecond())
            .expect("truncated OffsetDateTime")
    }

    fn floor_to_second(datetime: OffsetDateTime) -> OffsetDateTime {
        datetime
            .replace_millisecond(0)
            .expect("truncated OffsetDateTime")
    }

    #[cfg(test)]
    mod milli_tests {
        use time::macros::datetime;

        use super::super::millisecond;
        use super::*;

        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        struct Owned(#[serde(with = "millisecond")] OffsetDateTime);

        #[derive(Serialize)]
        #[serde(transparent)]
        struct Ref<'a>(#[serde(with = "millisecond")] &'a OffsetDateTime);

        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        struct OptionOwned(#[serde(with = "millisecond")] Option<OffsetDateTime>);

        #[test]
        fn deserialize_millisecond() {
            let actual: Owned =
                serde_json::from_str(r#""2022-01-01T19:00:10.123456789+07:00""#).unwrap();
            assert_eq!(actual.0, datetime!(2022-01-01 19:00:10.123+07:00));

            let actual: OptionOwned =
                serde_json::from_str(r#""2022-01-01T19:00:10.123456789+07:00""#).unwrap();
            assert_eq!(actual.0.unwrap(), datetime!(2022-01-01 19:00:10.123+07:00));
        }

        #[test]
        fn serialize_millisecond() {
            let datetime = datetime!(2022-01-01 19:00:10.123456789+07:00);

            let actual = serde_json::to_string(&Owned(datetime)).unwrap();
            assert_eq!(actual, r#""2022-01-01T19:00:10.123+07:00""#);

            let actual = serde_json::to_string(&Ref(&datetime)).unwrap();
            assert_eq!(actual, r#""2022-01-01T19:00:10.123+07:00""#);

            let actual = serde_json::to_string(&OptionOwned(Some(datetime))).unwrap();
            assert_eq!(actual, r#""2022-01-01T19:00:10.123+07:00""#);

            let actual = serde_json::to_string(&OptionOwned(None)).unwrap();
            assert_eq!(actual, "null");

            let actual =
                serde_json::to_string(&Owned(datetime!(2022-01-01 19:00:10.123456789+00:00)))
                    .unwrap();
            assert_eq!(actual, r#""2022-01-01T19:00:10.123Z""#);
        }
    }

    #[cfg(test)]
    mod second_tests {
        use time::macros::datetime;

        use super::super::second;
        use super::*;

        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        struct Owned(#[serde(with = "second")] OffsetDateTime);

        #[derive(Serialize)]
        #[serde(transparent)]
        struct Ref<'a>(#[serde(with = "second")] &'a OffsetDateTime);

        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        struct OptionOwned(#[serde(with = "second")] Option<OffsetDateTime>);

        #[test]
        fn deserialize_second() {
            let actual: Owned =
                serde_json::from_str(r#""2022-01-01T19:00:10.123456789+07:00""#).unwrap();
            assert_eq!(actual.0, datetime!(2022-01-01 19:00:10+07:00));

            let actual: OptionOwned =
                serde_json::from_str(r#""2022-01-01T19:00:10.123456789+07:00""#).unwrap();
            assert_eq!(actual.0.unwrap(), datetime!(2022-01-01 19:00:10+07:00));
        }

        #[test]
        fn serialize_second() {
            let datetime = datetime!(2022-01-01 19:00:10.123456789+07:00);

            let actual = serde_json::to_string(&Owned(datetime)).unwrap();
            assert_eq!(actual, r#""2022-01-01T19:00:10+07:00""#);

            let actual = serde_json::to_string(&Ref(&datetime)).unwrap();
            assert_eq!(actual, r#""2022-01-01T19:00:10+07:00""#);

            let actual = serde_json::to_string(&OptionOwned(Some(datetime))).unwrap();
            assert_eq!(actual, r#""2022-01-01T19:00:10+07:00""#);

            let actual = serde_json::to_string(&OptionOwned(None)).unwrap();
            assert_eq!(actual, "null");

            let actual =
                serde_json::to_string(&Owned(datetime!(2022-01-01 19:00:10.123456789+00:00)))
                    .unwrap();
            assert_eq!(actual, r#""2022-01-01T19:00:10Z""#);
        }
    }
}
