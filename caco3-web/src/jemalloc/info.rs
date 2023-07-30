use byte_unit::AdjustedByte;
use serde::Serialize;

#[derive(Serialize)]
pub struct JemallocInfo {
    options: Options,
    stats: Stats,
}

#[derive(Serialize)]
struct Stats {
    // these two are the most interested
    allocated: AdjustedByte,
    resident: AdjustedByte,
    // other values
    active: AdjustedByte,
    mapped: AdjustedByte,
    metadata: AdjustedByte,
    retained: AdjustedByte,
}

#[derive(Serialize)]
struct Options {
    background_thread: Option<BackgroundThread>,
    number_of_arenas: u32,
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
        use byte_unit::Byte;
        fn byte_from_usize(n: usize) -> Option<AdjustedByte> {
            Some(Byte::from_bytes(n.try_into().ok()?).get_appropriate_unit(true))
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
