use crate::{player::Player, utils::Version};
use bytes::{Buf, Bytes};
use log::{error, warn};
use ssbm_utils::prelude::*;
use ssbm_utils::types::*;
use ssbm_utils::{pos, stick_pos};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PreFrame {
    pub frame_index: i32,
    pub port: u8,
    pub nana: bool,
    pub random_seed: u32,
    pub action_state: State,
    pub position: Position,
    pub orientation: f32,
    pub joystick: StickPos,
    pub cstick: StickPos,
    pub engine_trigger: f32,
    pub engine_buttons: u32,
    pub controller_buttons: u16,
    pub controller_l: f32,
    pub controller_r: f32,
    pub raw_stick_x: Option<i8>,
    pub percent: Option<f32>,
    pub raw_stick_y: Option<i8>,
}

impl PreFrame {
    pub fn validate(&self) {
        let port = self.port;
        let idx = self.frame_index;
        if matches!(self.action_state, State::Unknown(_)) {
            warn!(
                "[Frame {idx}, Port {}] Unknown state: {}",
                Port::from_repr(port).unwrap(), self.action_state
            );
        }
        if !matches!(self.orientation, -1.0 | 0.0 | 1.0) {
            warn!(
                "[Frame {idx}, Port {}] Invalid orientation raw value: {}",
                Port::from_repr(port).unwrap(), self.orientation
            );
        }
        if !(-1.0..=1.0).contains(&self.joystick.x) || !(-1.0..=1.0).contains(&self.joystick.y) {
            warn!(
                "[Frame {idx}, Port {}] Invalid joystick coordinates: {}",
                Port::from_repr(port).unwrap(), self.joystick
            );
        }
        if !(-1.0..=1.0).contains(&self.cstick.x) || !(-1.0..=1.0).contains(&self.cstick.y) {
            warn!(
                "[Frame {idx}, Port {}] Invalid cstick coordinates: {}",
                Port::from_repr(port).unwrap(), self.cstick
            );
        }
        if !(0.0..=1.0).contains(&self.engine_trigger) {
            warn!(
                "[Frame {idx}, Port {}] Invalid engine trigger value: {}",
                Port::from_repr(port).unwrap(), self.engine_trigger
            );
        }
        if self.engine_buttons & 0x7F00_E080 != 0 {
            warn!(
                "[Frame {idx}, Port {}] Invalid bits set in engine buttons: {:032b}",
                Port::from_repr(port).unwrap(), self.engine_buttons
            );
        }
        if !(0.0..=1.0).contains(&self.controller_l) {
            warn!(
                "[Frame {idx}, Port {}] Invalid controller L value: {}",
                Port::from_repr(port).unwrap(), self.controller_l
            );
        }
        if !(0.0..=1.0).contains(&self.controller_r) {
            warn!(
                "[Frame {idx}, Port {}] Invalid controller R value: {}",
                Port::from_repr(port).unwrap(), self.controller_r
            );
        }
        // if self.raw_stick_x.is_some_and(|x| !(-110..=110).contains(&x)) {
        //     warn!(
        //         "[Frame {idx}, Port{port}] Unexpected raw stick x: {}. Expected stick value in range -110 through 110",
        //         self.raw_stick_x.unwrap()
        //     );
        // }
        if self.percent.is_some_and(|p| !(0.0..1000.0).contains(&p)) {
            warn!(
                "[Frame {idx}, Port {}] Invalid percent: {}",
                Port::from_repr(port).unwrap(), self.percent.unwrap()
            );
        }
        // if self.raw_stick_y.is_some_and(|y| !(-110..=110).contains(&y)) {
        //     warn!(
        //         "[Frame {idx}, Port{port}] Unexpected raw stick y: {}. Expected stick value in range -110 through 110",
        //         self.raw_stick_y.unwrap()
        //     );
        // }
    }

    pub fn new(mut stream: Bytes, version: Version, players: &[Player; 4]) -> Self {
        let frame_index = stream.get_i32();
        let port = stream.get_u8();
        let follower = stream.get_u8() == 1;

        let character = players[port as usize].character;
        if character != Character::IceClimbers && follower {
            error!(
                "[Frame {frame_index}, Port {port}] Has Nana frame but is playing {}",
                character
            );
        }
        let result = Self {
            frame_index,
            port,
            nana: follower,
            random_seed: stream.get_u32(),
            action_state: {
                let state = State::from_state_and_char(stream.get_u16(), Some(character));
                if let State::Unknown(x) = state {
                    if character == Character::Zelda {
                        State::from_state_and_char(stream.get_u16(), Some(Character::Sheik))
                    } else {
                        State::Unknown(x)
                    }
                } else {
                    state
                }
            },
            position: pos!(stream.get_f32(), stream.get_f32()),
            orientation: stream.get_f32(),
            joystick: stick_pos!(stream.get_f32(), stream.get_f32()),
            cstick: stick_pos!(stream.get_f32(), stream.get_f32()),
            engine_trigger: stream.get_f32(),
            engine_buttons: stream.get_u32(),
            controller_buttons: stream.get_u16(),
            controller_l: stream.get_f32(),
            controller_r: stream.get_f32(),
            raw_stick_x: version.at_least(1, 2, 0).then(|| stream.get_i8()),
            percent: version.at_least(1, 4, 0).then(|| stream.get_f32()),
            raw_stick_y: version.at_least(3, 15, 0).then(|| stream.get_i8()),
        };

        result.validate();

        result
    }
}
