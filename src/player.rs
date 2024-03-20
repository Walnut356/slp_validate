use ssbm_utils::prelude::*;
use strum::FromRepr;

use crate::{game_start::UCFToggles, utils::Tournament};

#[derive(Debug, Clone, Copy, PartialEq, FromRepr, Default)]
#[repr(u8)]
pub enum PlayerType {
    Human = 0,
    CPU = 1,
    Demo = 2,
    #[default]
    Empty = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, FromRepr, Default)]
#[repr(u8)]
pub enum TeamShade {
    #[default]
    Normal,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, FromRepr, Default)]
#[repr(u8)]
pub enum TeamID {
    #[default]
    Red,
    Blue,
    Green,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Player {
    pub port: Port,
    pub player_type: PlayerType,
    pub character: Character,
    pub starting_stocks: u8,
    pub costume: Costume,
    pub team_shade: TeamShade,
    pub handicap: u8,
    pub team_id: TeamID,
    pub bitfield: u8,
    pub cpu_level: u8,
    pub damage_start: u16,
    pub damage_spawn: u16,
    pub offense_ratio: f32,
    pub defense_ratio: f32,
    pub model_scale: f32,
    pub ucf: Option<UCFToggles>,
    pub connect_code: Option<String>,
    pub display_name: Option<String>,
}

impl Tournament for Player {
    fn is_legal(&self) -> bool {
        if self.player_type == PlayerType::Empty {
            return true;
        }

        self.player_type == PlayerType::Human
            && !matches!(
                self.character,
                Character::MasterHand
                    | Character::GigaBowser
                    | Character::WireframeFemale
                    | Character::WireframeMale
            )
            && self.starting_stocks == 4
            && self.handicap == 0
            && self.bitfield >> 1 == 0
            && self.damage_spawn == 0
            && self.damage_start == 0
    }
}
