#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    EnumString,
    Display,
    Serialize,
    Deserialize,
)]
pub enum Currency {
    JPY,
    USD,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JPY {}
