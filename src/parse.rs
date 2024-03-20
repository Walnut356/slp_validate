use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, ensure, Result};
use bytes::{Buf, Bytes};
use log::{debug, error, info, log_enabled, trace, warn, Level};
use serde_json::{Map, Value};
use ssbm_utils::prelude::Character;
use strum::FromRepr;

use crate::{
    frame::{FrameEnd, FrameStart}, game_end::GameEnd, game_start::GameStart, itemframe::ItemFrame, player::PlayerType, postframe::PostFrame, preframe::PreFrame, ubjson, utils::ParseError
};

pub fn parse(path: &str) {
    let f_path = Path::new(path);
    if f_path.is_file() {
        info!("Parsing file {path}");

        if let Err(e) = validate_game(f_path.to_path_buf()) {
            error!("{e}");
        }
        return;
    }
    if f_path.is_dir() {
        info!("Parsing directory {path}");

        let files: Vec<PathBuf> = fs::read_dir(f_path)
            .unwrap()
            .filter_map(|file| {
                if let Ok(entry) = file {
                    let path = entry.path();
                    if path.is_file() && path.extension().unwrap() == "slp" {
                        Some(path)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        info!("Found {} files", files.len());
        for file in files {
            info!("Validating {}", file.to_str().unwrap());
        }
        return;
    }
    error!("invalid file path: {f_path:?}")
}

fn get_file_contents(path: &Path) -> Result<Bytes> {
    let mut f = File::open(path)?;
    let file_length = f.metadata()?.len() as usize;
    // #[cfg(debug_assertions)]
    // dbg!(file_length);
    let mut file_data = vec![0; file_length];
    f.read_exact(&mut file_data).unwrap();

    Ok(Bytes::from(file_data))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, Default, Hash)]
#[repr(u8)]
enum EventType {
    EventPayloads = 0x35,
    GameStart = 0x36,
    PreFrame = 0x37,
    PostFrame = 0x38,
    GameEnd = 0x39,
    FrameStart = 0x3A,
    Item = 0x3B,
    FrameEnd = 0x3C,
    GeckoList = 0x3D,
    MessageSplitter = 0x10,
    #[default]
    None = 0x00,
}

fn get_event_sizes(file: &mut Bytes) -> Result<HashMap<EventType, u16>> {
    let code = EventType::from_repr(file.get_u8()).unwrap();
    ensure!(
        code == EventType::EventPayloads,
        ParseError::Value(
            format!("{:?}", EventType::EventPayloads),
            format!("{:?}", code)
        )
    );

    let payloads_size = file.get_u8();

    ensure!(
        (payloads_size - 1) % 3 == 0,
        anyhow!("EventPayloads length invalid")
    );

    let mut event_map = HashMap::default();

    for _ in (0..(payloads_size - 1)).step_by(3) {
        let event = EventType::from_repr(file.get_u8()).unwrap();
        let size = file.get_u16();
        event_map.insert(event, size);
    }

    Ok(event_map)
}

fn expect_bytes(stream: &mut Bytes, expected: &[u8], message: &str) -> std::io::Result<()> {
    let actual = stream.get(0..expected.len()).unwrap();
    if expected == actual {
        stream.advance(expected.len());
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Expected {message}: {expected:?}, got {actual:?}"),
        ))
    }
}

fn validate_game(path: PathBuf) -> Result<()> {
    let file_data = get_file_contents(&path)?;
    let mut stream = file_data.slice(..);

    expect_bytes(
        &mut stream,
        &[
            0x7b, 0x55, 0x03, 0x72, 0x61, 0x77, 0x5b, 0x24, 0x55, 0x23, 0x6c,
        ],
        "Slippi header",
    )?;

    let raw_length = stream.get_u32() as u64 + 15;
    trace!("Raw length: {raw_length}");

    // ----------------------------------- metadata block ----------------------------------- //
    let mut temp_meta = file_data.slice(raw_length as usize..);

    expect_bytes(
        &mut temp_meta,
        // `metadata` key & type ("U\x08metadata{")
        &[
            0x55, 0x08, 0x6d, 0x65, 0x74, 0x61, 0x64, 0x61, 0x74, 0x61, 0x7b,
        ],
        "metadata header",
    )?;

    let mut frame_count: usize = 0;

    let metadata = ubjson::to_map(&mut temp_meta.reader())?;
    if let serde_json::Value::Number(lastframe) = &metadata["lastFrame"] {
        // duration, in frames, is translated to seconds. 123 is subtracted from the frame count
        // to match the duration to the in-game timer. The total frame count is easily
        // found from player.frames.len()
        let last = lastframe.as_i64().unwrap();
        frame_count = (last + 124) as usize;
    };

    trace!("Frame count: {frame_count}");

    if let serde_json::Value::String(start_at) = &metadata["startAt"] {
        trace!("Date: {start_at}")
    }

    let event_sizes = get_event_sizes(&mut stream)?;

    expect_bytes(
        &mut stream,
        &[EventType::GameStart as u8],
        "game start command byte",
    )?;

    let raw_start = stream.slice(0..event_sizes[&EventType::GameStart] as usize);
    stream.advance(event_sizes[&EventType::GameStart] as usize);

    let (game_start, version, mut players) = GameStart::parse(raw_start)?;

    info!("Parser max version: 3.16.0, Replay version: {version}");

    let mut event = EventType::None;
    let mut pos = file_data.len() - stream.len();

    let mut fstart = FrameStart::default();
    // tiny workaround due to debug rollback check
    fstart.frame_idx = -123;
    let mut pre = PreFrame::default();
    let mut post = PostFrame::default();
    let mut fend = FrameEnd::default();
    let mut item = ItemFrame::default();
    let mut game_end = None;

    let mut event_order = vec![EventType::FrameStart];

    for player in &players {
        if matches!(player.player_type, PlayerType::CPU | PlayerType::Human) {
            event_order.push(EventType::PreFrame);
            if player.character == Character::IceClimbers {
                event_order.push(EventType::PreFrame);
            }
        }
    }

    event_order.push(EventType::Item);

    for player in &players {
        if matches!(player.player_type, PlayerType::CPU | PlayerType::Human) {
            event_order.push(EventType::PostFrame);
            if player.character == Character::IceClimbers {
                event_order.push(EventType::PostFrame);
            }
        }
    }

    event_order.push(EventType::FrameEnd);
    let mut order_idx = 0;
    let mut need_sync = false;


    while pos < raw_length as usize && event != EventType::GameEnd && stream.has_remaining(){
        let code = stream.get_u8();
        event = EventType::from_repr(code).unwrap_or_default();
        /* EventType::None allows the parser to continue working on newer replays (with possible
        new events). During testing all events must be accounted for, so any EventType::Nones
        are likely a misalignment of my slices */
        if event == EventType::None {
            warn!("Unknown event type: {code}");
        }
        let size = event_sizes[&event] as usize;

        match event {
            EventType::FrameStart => {
                let old_frame = fstart.frame_idx;
                fstart = FrameStart::new(stream.slice(..size), version);
                if need_sync || event_order[order_idx] != EventType::FrameStart {
                    let expected = match event_order[order_idx] {
                        EventType::Item => "EventType::Item or EventType::PostFrame".to_owned(),
                        x => format!("EventType::{:?}", x),
                    };
                    error!("[File pos: {}] Unexpected event ordering. Expected {} for frame {}, got EventType::FrameStart for frame {}", pos, expected, old_frame, fstart.frame_idx);
                    order_idx = 0;
                    need_sync = false;
                }
                if fstart.frame_idx - old_frame > 1 || fstart.frame_idx - old_frame < -10 {
                    error!("[File pos: {}] Unexpected frame ordering. Previous frame was index {}, current frame is index {}", pos, old_frame, fstart.frame_idx);
                }
                if fstart.frame_idx < old_frame {
                    debug!("[File pos: {}] Rollback from frame {} to frame {}", pos, old_frame, fstart.frame_idx);
                }
                order_idx += 1;
            },
            EventType::PreFrame => {
                pre = PreFrame::new(stream.slice(..size), version, &players);
                if !need_sync && event_order[order_idx] != EventType::PreFrame {
                    need_sync = true;
                    let expected = match event_order[order_idx] {
                        EventType::Item => "EventType::Item or EventType::PostFrame".to_owned(),
                        x => format!("{:?}", x),
                    };
                    error!("[File pos: {}] Unexpected event ordering. Expected {} for frame {}, got EventType::PreFrame for frame {}", pos, expected, fstart.frame_idx, pre.frame_index)
                }
                order_idx += 1;
            },
            EventType::PostFrame => {
                post = PostFrame::new(stream.slice(..size), version);
                if !need_sync && !matches!(event_order[order_idx], EventType::PostFrame | EventType::Item) {
                    need_sync = true;
                    let expected = match event_order[order_idx] {
                        EventType::Item => "EventType::Item or EventType::PostFrame".to_owned(),
                        x => format!("{:?}", x),
                    };
                    error!("[File pos: {}] Unexpected event ordering. Expected {} for frame {}, got EventType::PostFrame for frame {}", pos, expected, fstart.frame_idx, pre.frame_index)
                }
                // handling for item frames as they aren't guaranteed to exist
                if event_order[order_idx] == EventType::Item {
                    order_idx += 2;
                } else {
                    order_idx += 1;
                }

            },
            EventType::FrameEnd => {
                fend = FrameEnd::new(stream.slice(..size), version);
                if !need_sync && !matches!(event_order[order_idx], EventType::FrameEnd) {
                    need_sync = true;
                    error!("[File pos: {}] Unexpected event ordering. Expected {:?} for frame {}, got EventType::FrameEnd for frame {}", pos, event_order[order_idx], fstart.frame_idx, pre.frame_index)
                }
                order_idx = 0;
            },
            // Item frames don't increment counter as there can be 0 or up to 15
            EventType::Item => {
                item = ItemFrame::new(stream.slice(..size), version);
                if !need_sync && !matches!(event_order[order_idx], EventType::Item) {
                    need_sync = true;
                    error!("[File pos: {}] Unexpected event ordering. Expected {:?} for frame {}, got EventType::Item for frame {}", pos, event_order[order_idx], fstart.frame_idx, pre.frame_index)
                }

            },
            EventType::GameEnd => {
                if game_end.is_some() {
                    warn!("[File pos: {}] Duplicate game end event", pos);
                }
                game_end = Some(GameEnd::new(stream.slice(..size), version))
            },
            _ => (),
        }
        stream.advance(event_sizes[&event] as usize);
        pos = file_data.len() - stream.len();
    }

    Ok(())
}
