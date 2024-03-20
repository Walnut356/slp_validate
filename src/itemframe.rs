use crate::utils::Version;
use bytes::{Buf, Bytes};
use log::warn;
use ssbm_utils::prelude::*;
use ssbm_utils::types::*;
use ssbm_utils::{pos, vel};


#[derive(Debug, Default, Clone, PartialEq)]
pub struct ItemFrame {
    pub frame_index: i32,
    /// The ID corresponding to the type of item that this frame data is about.
    pub item_id: u16,
    pub state: u8,
    pub orientation: f32,
    pub velocity: Velocity,
    pub position: Position,
    pub damage_taken: u16,
    pub expiration_timer: f32,
    /// A unique ID artificially given to each projectile to help differentiate it from other items spawned
    /// during the same game.
    pub spawn_id: u32,
    pub missile_type: Option<u8>,
    pub turnip_type: Option<u8>,
    pub launched: Option<bool>,
    pub charge_power: Option<u8>,
    pub owner: Option<i8>,
    pub instance_id: Option<u16>,
}

impl ItemFrame {
    pub fn new(mut stream: Bytes, version: Version) -> Self {
        let result = Self {
            frame_index: stream.get_i32(),
            item_id: stream.get_u16(),
            state: stream.get_u8(),
            orientation: stream.get_f32(),
            velocity: vel!(stream.get_f32(), stream.get_f32()),
            position: pos!(stream.get_f32(), stream.get_f32()),
            damage_taken: stream.get_u16(),
            expiration_timer: stream.get_f32(),
            spawn_id: stream.get_u32(),
            missile_type: version.at_least(3, 2, 0).then(|| stream.get_u8()),
            turnip_type: version.at_least(3, 2, 0).then(|| stream.get_u8()),
            launched: version.at_least(3, 2, 0).then(|| stream.get_u8() != 0),
            charge_power: version.at_least(3, 2, 0).then(|| stream.get_u8()),
            owner: version.at_least(3, 6, 0).then(|| stream.get_i8()),
            instance_id: version.at_least(3, 16, 0).then(|| stream.get_u16()),
        };

        result.validate();

        result
    }

    pub fn validate(&self) {
        let idx = self.frame_index;
        if Item::from_repr(self.item_id).is_none() {
            warn!(
                "[Frame {idx}, Item] Invalid item id: {}",
                self.item_id);
        }

    }
}