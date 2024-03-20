use std::time::Duration;

use anyhow::{anyhow, Result};
use bytes::{Buf, Bytes};
use encoding_rs::SHIFT_JIS;
use log::warn;
use ssbm_utils::enums::{character::Character, stage::StageID, Port};
use strum::{Display, FromRepr, IntoStaticStr};

use crate::{
    player::{Player, PlayerType, TeamID, TeamShade},
    utils::Version,
};

#[derive(Debug, Clone, Copy, PartialEq, FromRepr, Default)]
#[repr(u8)]
pub enum Mode {
    VS = 2,
    Online = 8,
    #[default]
    Unknown = 0,
}

#[derive(Debug, Clone, Copy, PartialEq, FromRepr, IntoStaticStr, Default, Display)]
#[repr(u8)]
pub enum MatchType {
    // ascii character values for u, r, d
    Unranked = 117,
    Ranked = 114,
    Direct = 100,
    #[default]
    Unknown = 0,
}

#[derive(Debug, Clone, Copy, PartialEq, FromRepr, Default)]
#[repr(u8)]
pub enum ControllerFix {
    Off = 0,
    #[default] // this has more or less been true since like 2018
    UCF = 1,
    Dween = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, FromRepr)]
pub enum Timer {
    #[default]
    None,
    CountDown,
    CountUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, FromRepr)]
pub enum VsMode {
    #[default]
    Time,
    Stock,
    Coin,
    Bonus,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct UCFToggles {
    pub dashback: ControllerFix,
    pub shield_drop: ControllerFix,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameStart {
    /// Random seed at the start of the match
    pub random_seed: u32,
    /// True if teams mode is active, regardless of the number of players in the match
    pub teams: bool,
    /// Simple stage ID. For stage data (blast zones, ledge locations, etc.), cast into `Stage`
    pub stage: StageID,
    /// The timer setting for the match, will usually be 8 minutes (480s)
    pub timer: Duration,
    /// Damage ratio in the settings menu, should almost always be 1.0
    pub damage_ratio: f32,
    /// True if PAL
    ///
    /// added v1.5.0
    pub pal: Option<bool>,
    /// True if stadium is frozen
    ///
    /// added v2.0.0
    pub frozen_stadium: Option<bool>,
    /// True if played on slippi netplay
    ///
    /// added v3.7.0
    pub netplay: Option<bool>,
    /// Match id, usually very similar to the default file name
    ///
    /// added v3.14.0
    pub match_id: String,
    /// Unranked, Ranked, Direct, or Unknown. Note that Doubles is not an option because this parser
    /// handles 1v1 replays only
    ///
    /// added v3.14.0
    pub match_type: MatchType,
    /// For the given match ID, this is Xth game played. Starts at 1
    ///
    /// added v3.14.0
    pub game_number: Option<u32>,
    /// For the given match ID, this is the Xth tiebreak game played. Will almost always be 0
    ///
    /// added v3.14.0
    pub tiebreak_number: Option<u32>,
}

impl GameStart {
    // the awkward return type here is because this will only ever be constructed internally, and because it will help
    // a LOT down the line to have the players contained in the top level Game object rather than the GameStart event.
    pub fn parse(mut raw: Bytes) -> Result<(Self, Version, [Player; 4])> {
        let version = Version::new(raw.get_u8(), raw.get_u8(), raw.get_u8());
        raw.advance(9); // skip past revision number, game bitfields 1-4 and bomb rain

        let is_teams = raw.get_u8() != 0;
        raw.advance(5); // skip item spawn rate and self destruct score value

        let stage = StageID::from_repr(raw.get_u16()).unwrap();

        // timer value is given in seconds, can only be changed by full-minute increments in-game
        let timer_length = Duration::from_secs(raw.get_u32() as u64);
        raw.advance(28); // skip past item spawn bitfields

        let damage_ratio = raw.get_f32();
        raw.advance(44); // skip to player

        let mut players: [Player; 4] = [
            Player::default(),
            Player::default(),
            Player::default(),
            Player::default(),
        ];

        for i in 0..4 {
            let character = Character::try_from_css(raw.get_u8()).unwrap_or_default();
            let player_type = PlayerType::from_repr(raw.get_u8());
            if player_type.is_none() {
                warn!("Invalid player type for player {i}");
            }
            let player_type = player_type.unwrap_or_default();

            let starting_stocks = raw.get_u8();
            let costume = character.get_costume(raw.get_u8());
            let team_shade = TeamShade::from_repr(raw.get_u8());
            if matches!(player_type, PlayerType::Human | PlayerType::CPU) && team_shade.is_none() {
                warn!("Invalid team shade for player {i}");
            }
            let handicap = raw.get_u8();
            let team_id = TeamID::from_repr(raw.get_u8());
            if matches!(player_type, PlayerType::Human | PlayerType::CPU) && team_id.is_none() {
                warn!("Invalid team ID for player {i}");
            }
            let bitfield = raw.get_u8();
            let cpu_level = raw.get_u8();
            let damage_start = raw.get_u16();
            let damage_spawn = raw.get_u16();
            let offense_ratio = raw.get_f32();
            let defense_ratio = raw.get_f32();
            let model_scale = raw.get_f32();
            raw.advance(11);

            players[i] = Player {
                port: Port::from_repr(i as u8).unwrap(),
                player_type,
                character,
                starting_stocks,
                costume,
                team_shade: team_shade.unwrap_or_default(),
                handicap,
                team_id: team_id.unwrap_or_default(),
                bitfield,
                cpu_level,
                damage_start,
                damage_spawn,
                offense_ratio,
                defense_ratio,
                model_scale,
                connect_code: None,
                display_name: None,
                ucf: None,
            };
        }

        raw.advance(72); // skip past "players" 5 and 6

        let random_seed = raw.get_u32();

        // Null out potentially uninitialized values:
        let is_pal = None;
        let is_frozen_stadium = None;
        let is_netplay = None;
        let match_id = "".to_string();
        let match_type = MatchType::Unknown;
        let game_number = None;
        let tiebreak_number = None;

        let mut result = GameStart {
            random_seed,
            teams: is_teams,
            stage,
            timer: timer_length,
            pal: is_pal,
            frozen_stadium: is_frozen_stadium,
            netplay: is_netplay,
            match_id,
            match_type,
            game_number,
            tiebreak_number,
            damage_ratio,
        };

        if !version.at_least(1, 0, 0) {
            return Ok((result, version, players));
        }

        for player in players.iter_mut() {
            let dashback = ControllerFix::from_repr(raw.get_u32() as u8).unwrap();
            let shield_drop = ControllerFix::from_repr(raw.get_u32() as u8).unwrap();
            player.ucf = Some(UCFToggles {
                dashback,
                shield_drop,
            });
        }

        if !version.at_least(1, 3, 0) {
            return Ok((result, version, players));
        }

        raw.advance(64); // skip past in-game tags

        if !version.at_least(1, 5, 0) {
            return Ok((result, version, players));
        }

        result.pal = Some(raw.get_u8() != 0);

        if !version.at_least(2, 0, 0) {
            return Ok((result, version, players));
        }

        result.frozen_stadium = Some(raw.get_u8() != 0);

        if !version.at_least(3, 7, 0) {
            return Ok((result, version, players));
        }

        raw.advance(1); // skip minor scene
        result.netplay = Some(raw.get_u8() == 8);

        if !version.at_least(3, 9, 0) {
            return Ok((result, version, players));
        }

        for player in players.iter_mut() {
            let mut dn_bytes = vec![0; 31];
            raw.copy_to_slice(&mut dn_bytes);
            let end = dn_bytes.iter().position(|&x| x == 0).unwrap_or(30);
            dn_bytes.truncate(end);
            let (display_name, _, _) = SHIFT_JIS.decode(&dn_bytes);
            player.display_name = Some(display_name.to_string());
        }

        for player in players.iter_mut() {
            let mut cc_bytes = vec![0; 10];
            raw.copy_to_slice(&mut cc_bytes);
            let end = cc_bytes.iter().position(|&x| x == 0).unwrap_or(10);
            cc_bytes.truncate(end);
            let (connect_code, _, _) = SHIFT_JIS.decode(&cc_bytes);
            // replace the full width hash symbol with the ascii variant so people can actually type them
            let adjusted = connect_code.replace('＃', "#");
            player.connect_code = Some(adjusted);
        }

        if !version.at_least(3, 11, 0) {
            return Ok((result, version, players));
        }

        raw.advance(29 * 4); // skip past slippi uid

        if !version.at_least(3, 12, 0) {
            return Ok((result, version, players));
        }

        raw.advance(1); // skip language option

        if !version.at_least(3, 14, 0) {
            return Ok((result, version, players));
        }

        let mut match_id_bytes = vec![0; 51];
        raw.copy_to_slice(&mut match_id_bytes);
        let end = match_id_bytes.iter().position(|&x| x == 0).unwrap_or(50);
        match_id_bytes.truncate(end);
        let match_id_len = match_id_bytes.len();
        result.match_id = String::from_utf8(match_id_bytes).unwrap();

        result.game_number = Some(raw.get_u32());
        result.tiebreak_number = Some(raw.get_u32());

        result.match_type = {
            if match_id_len > 5 {
                MatchType::from_repr(result.match_id.as_bytes()[5]).unwrap_or_default()
            } else {
                MatchType::Unknown
            }
        };

        Ok((result, version, players))
    }
}

impl Default for Version {
    /// Returns Version{0, 1, 0}, the first slippi release version
    #[inline]
    fn default() -> Self {
        Self {
            major: 0,
            minor: 1,
            build: 0,
        }
    }
}
