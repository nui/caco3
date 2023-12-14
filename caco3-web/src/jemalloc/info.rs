use std::fmt::Write;

use arrayvec::ArrayString;
use byte_unit::{Byte, UnitType};
use serde::ser::Error as _;
use serde::{Serialize, Serializer};

#[derive(Serialize)]
pub struct JemallocInfo {
    pub options: Options,
    pub stats: Stats,
}

#[derive(Serialize)]
pub struct Stats {
    // these two are the most interested
    #[serde(serialize_with = "serialize_byte")]
    pub allocated: Byte,
    #[serde(serialize_with = "serialize_byte")]
    pub resident: Byte,
    // other values
    #[serde(serialize_with = "serialize_byte")]
    pub active: Byte,
    #[serde(serialize_with = "serialize_byte")]
    pub mapped: Byte,
    #[serde(serialize_with = "serialize_byte")]
    pub metadata: Byte,
    #[serde(serialize_with = "serialize_byte")]
    pub retained: Byte,
}

fn serialize_byte<S>(this: &Byte, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut buffer: ArrayString<256> = ArrayString::new();
    let adjusted_byte = this.get_appropriate_unit(UnitType::Binary);
    write!(&mut buffer, "{adjusted_byte:.2}")
        .map_err(|_| S::Error::custom(format!("serialize adjusted byte: {adjusted_byte}")))?;
    serializer.serialize_str(buffer.as_str())
}

#[derive(Serialize)]
pub struct Options {
    pub background_thread: Option<BackgroundThread>,
    pub number_of_arenas: u32,
}

#[doc(hidden)]
#[derive(Serialize)]
pub struct BackgroundThread {
    pub enabled: bool,
    pub max: usize,
}

#[doc(hidden)]
pub struct JemallocRawData {
    // stats
    pub active_bytes: usize,
    pub allocated_bytes: usize,
    pub mapped_bytes: usize,
    pub metadata_bytes: usize,
    pub resident_bytes: usize,
    pub retained_bytes: usize,
    // options
    pub background_thread: Option<BackgroundThread>,
    pub number_of_arenas: u32,
}

impl JemallocInfo {
    pub fn from_raw(raw_data: JemallocRawData) -> Option<Self> {
        fn byte_from_usize(n: usize) -> Option<Byte> {
            Some(Byte::from_u64(n.try_into().ok()?))
        }
        let jemalloc = {
            let JemallocRawData {
                active_bytes,
                allocated_bytes,
                background_thread,
                mapped_bytes,
                metadata_bytes,
                number_of_arenas,
                resident_bytes,
                retained_bytes,
            } = raw_data;
            JemallocInfo {
                options: Options {
                    background_thread,
                    number_of_arenas,
                },
                stats: Stats {
                    active: byte_from_usize(active_bytes)?,
                    allocated: byte_from_usize(allocated_bytes)?,
                    mapped: byte_from_usize(mapped_bytes)?,
                    metadata: byte_from_usize(metadata_bytes)?,
                    resident: byte_from_usize(resident_bytes)?,
                    retained: byte_from_usize(retained_bytes)?,
                },
            }
        };
        Some(jemalloc)
    }
}
