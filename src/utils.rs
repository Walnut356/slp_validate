use std::fmt::Display;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Incorrect file type, expected '.slp', got {0}")]
    FileType(String),
    #[error("Replay must have exactly 2 human players")]
    PlayerCount,
    #[error("Expected {0}, got {1}")]
    Value(String, String),
}


pub trait Tournament {
    fn is_legal(&self) -> bool;
}

/// Slippi replay version, dictates what information is available in the replay.
///
/// Version release dates listed below. Note that date checks can be misleading due to incorrectly
/// set dates on consoles, as well as updating slippi version late. This is purely meant as a
/// reference to know how old the replay spec is (and thus, roughly how likely it is that the
/// average replay contains a desired piece of information).
///

///
/// | Version | Released    |
/// |---------|-------------|
/// | 0.1.0   | Unknown     |
/// | 0.2.0   | Unknown     |
/// | 1.0.0   | Jul 01 2018 |
/// | 1.2.0   | Aug 08 2018 |
/// | 1.3.0   | Jan 14 2019 |
/// | 1.4.0   | Jan 15 2019 |
/// | 1.5.0   | Feb 08 2019 |
/// | 2.0.0   | Mar 19 2019 |
/// | 2.1.0   | Apr 28 2019 |
/// | 2.2.0   | Jun 24 2019 |
/// | 3.0.0   | Oct 24 2019 |
/// | 3.2.0   | Jan 31 2020 |
/// | 3.3.0   | Feb 11 2020 |
/// | 3.5.0   | Jun 13 2020 |
/// | 3.6.0   | Jun 20 2020 |
/// | 3.7.0   | Jul 08 2020 |
/// | 3.8.0   | Dec 06 2020 |
/// | 3.9.0   | Feb 06 2021 |
/// | 3.10.0  | Jan 26 2022 |
/// | 3.11.0  | Jan 30 2022 |
/// | 3.12.0  | Feb 07 2022 |
/// | 3.13.0  | Aug 30 2022 |
/// | 3.14.0  | Nov 04 2022 |
/// | 3.15.0  | May 27 2023 |
/// | 3.16.0  | Sep 20 2023 |
///
/// Some noteable dates in the Slippi ecosystem:
///
/// * The first public release was Jun 18 2018
/// * Slippi was made the main dolphin version on Anther's Ladder Sep 29 2018
/// * Rollback netplay was released Jun 22 2020
/// * Unranked MMR was released Jan 19 2021
/// * Doubles matchmaking was released May 2 2021
/// * Ranked was released Dec 12 2022
///
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
}

impl Version {
    #[inline]
    pub fn new(major: u8, minor: u8, build: u8) -> Self {
        Self {
            major,
            minor,
            build,
        }
    }

    #[inline]
    /// Returns true if self is at least (greater than or equal to) the given version
    pub fn at_least(&self, major: u8, minor: u8, build: u8) -> bool {
        *self
            >= Version {
                major,
                minor,
                build,
            }
    }

    #[inline]
    pub fn as_u32(&self) -> u32 {
        u32::from_be_bytes([self.major, self.minor, self.build, 0])
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.build)
    }
}