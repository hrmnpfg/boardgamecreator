use serde::{Deserialize, Serialize};
use std::ops::Not;

//Player
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Player {
  Black,
  White,
}

impl Player {
  pub fn white() -> Player {
    return Player::White;
  }

  pub fn from_str(s: &str) -> Option<Player> {
    match s {
      "White" => Some(Player::White),
      "Black" => Some(Player::Black),
      _ => None,
    }
  }

  pub fn as_str(&self) -> &str {
    match self {
      Player::Black => "Black",
      Player::White => "White",
    }
  }
}

impl Default for Player {
  fn default() -> Self {
    Player::White
  }
}

impl Not for Player {
  type Output = Self;

  fn not(self) -> Self::Output {
    match self {
      Self::White => Self::Black,
      Self::Black => Self::White,
    }
  }
}
