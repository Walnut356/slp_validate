use bytes::{Buf, Bytes};

use crate::utils::Version;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameStart {
    pub frame_idx: i32,
    pub frame_counter: Option<u32>,
}

impl FrameStart {
    pub fn new(mut stream: Bytes, version: Version) -> FrameStart {
        let frame_idx = stream.get_i32();
        // random seed
        stream.get_u32();
        let frame_counter = version.at_least(3, 10, 0).then(|| stream.get_u32());
        FrameStart {
            frame_idx,
            frame_counter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameEnd {
    pub frame_idx: i32,
    pub latest_finalized: Option<i32>,
}

impl FrameEnd {
    pub fn new(mut stream: Bytes, version: Version) -> FrameEnd {
        let frame_idx = stream.get_i32();
        let latest_finalized = if version.at_least(3, 7, 0) {
            Some(stream.get_i32())
        } else {
            None
        };
        FrameEnd {
            frame_idx,
            latest_finalized,
        }
    }
}
