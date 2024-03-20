use crate::utils::Version;
use bytes::{Buf, Bytes};
use log::warn;
use ssbm_utils::prelude::*;
use ssbm_utils::types::*;
use ssbm_utils::{pos, vel};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PostFrame {
    pub frame_index: i32,
    pub port: u8,
    pub nana: bool,
    pub character: u8,
    pub action_state: u16,
    pub position: Position,
    pub orientation: f32,
    pub percent: f32,
    pub shield_health: f32,
    pub last_attack_landed: u8,
    pub combo_count: u8,
    pub last_hit_by: u8,
    pub stocks: u8,
    pub state_frame: Option<f32>,
    pub flags: Option<u64>,
    pub misc_as: Option<f32>,
    pub is_grounded: Option<bool>,
    pub last_ground_id: Option<u16>,
    pub jumps_remaining: Option<u8>,
    pub l_cancel: Option<u8>,
    pub hurtbox_state: Option<u8>,
    pub air_velocity: Option<Velocity>,
    pub knockback: Option<Velocity>,
    pub ground_velocity: Option<Velocity>,
    pub hitlag_remaining: Option<f32>,
    pub animation_index: Option<u32>,
    pub instance_hit_by: Option<u16>,
    pub instance_id: Option<u16>,
}

impl PostFrame {
    pub fn new(mut stream: Bytes, version: Version) -> Self {
        let mut y_speed = 0.0;
        let result = Self {
            frame_index: stream.get_i32(),
            port: stream.get_u8(),
            nana: stream.get_u8() != 0,
            character: stream.get_u8(),
            action_state: stream.get_u16(),
            position: pos!(stream.get_f32(), stream.get_f32()),
            orientation: stream.get_f32(),
            percent: stream.get_f32(),
            shield_health: stream.get_f32(),
            last_attack_landed: stream.get_u8(),
            combo_count: stream.get_u8(),
            last_hit_by: stream.get_u8(),
            stocks: stream.get_u8(),
            state_frame: version.at_least(0, 2, 0).then(|| stream.get_f32()),
            flags: version.at_least(2, 0, 0).then(|| {
                (stream.get_u8() as u64)
                    | ((stream.get_u8() as u64) << 8)
                    | ((stream.get_u8() as u64) << 16)
                    | ((stream.get_u8() as u64) << 24)
                    | ((stream.get_u8() as u64) << 32)
            }),
            misc_as: version.at_least(2, 0, 0).then(|| stream.get_f32()),
            is_grounded: version.at_least(2, 0, 0).then(|| stream.get_u8() == 0),
            last_ground_id: version.at_least(2, 0, 0).then(|| stream.get_u16()),
            jumps_remaining: version.at_least(2, 0, 0).then(|| stream.get_u8()),
            l_cancel: version.at_least(2, 0, 0).then(|| stream.get_u8()),
            hurtbox_state: version.at_least(3, 1, 0).then(|| stream.get_u8()),
            air_velocity: version.at_least(3, 5, 0).then(|| {
                let x_speed = stream.get_f32();
                y_speed = stream.get_f32();
                vel!(x_speed, y_speed)
            }),
            knockback: version
                .at_least(3, 5, 0)
                .then(|| vel!(stream.get_f32(), stream.get_f32())),
            ground_velocity: version
                .at_least(3, 5, 0)
                .then(|| vel!(stream.get_f32(), y_speed)),
            hitlag_remaining: version.at_least(3, 8, 0).then(|| stream.get_f32()),
            animation_index: version.at_least(3, 11, 0).then(|| stream.get_u32()),
            instance_hit_by: version.at_least(3, 16, 0).then(|| stream.get_u16()),
            instance_id: version.at_least(3, 16, 0).then(|| stream.get_u16()),
        };

        result.validate();

        result
    }

    pub fn validate(&self) {
        let idx = self.frame_index;
        let port = self.port;
        let character = Character::try_from_internal(self.character).unwrap();
        if self.nana && self.character != Character::Nana.as_internal() {
            warn!(
                "[Frame {idx}, Port{port}] Nana frame for non-nana character: {}",
                self.character
            );
        }
        if let State::Unknown(x) = State::from_state_and_char(
            self.action_state,
            Some(character),
        ) {
            warn!("[Frame {idx}, Port{port}] Unknown state ID '{x}' for character {character}",);
        }
        if !matches!(self.orientation, -1.0 | 0.0 | 1.0) {
            warn!(
                "[Frame {idx}, Port{port}] Invalid orientation raw value: {}",
                self.orientation
            );
        }
        if !(0.0..1000.0).contains(&self.percent) {
            warn!(
                "[Frame {idx}, Port{port}] Invalid percent: {}",
                self.percent
            );
        }
        if !(0.0..=60.0).contains(&self.shield_health) {
            warn!(
                "[Frame {idx}, Port{port}] Invalid shield health: {}",
                self.shield_health,
            )
        }
        if Attack::from_repr(self.last_attack_landed).is_none() {
            warn!(
                "[Frame {idx}, Port{port}] Invalid attack ID: {}",
                self.last_attack_landed
            );
        }
        if self.flags.is_some_and(|f| f >> 40 != 0) {
            warn!(
                "[Frame {idx}, Port{port}] Invalid flag bits set: {:040b}",
                self.flags.unwrap());
        }
        if self.l_cancel.is_some_and(|l| l > 2) {
            warn!(
                "[Frame {idx}, Port{port}] Invalid l cancel value: {}",
                self.l_cancel.unwrap());
        }
        if self.hurtbox_state.is_some_and(|h| h > 2) {
            warn!(
                "[Frame {idx}, Port{port}] Invalid hurtbox value: {}",
                self.hurtbox_state.unwrap());
        }
    }


}
