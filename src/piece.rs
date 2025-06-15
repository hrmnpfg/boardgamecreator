use crate::player::Player;
use crate::PMove;
use crate::RuleExpression;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

///figura
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Piece {
  pub id: String,
  pub name: String,
  pub deathrattle: Option<Vec<RuleExpression>>,
  pub battlecry: Option<Vec<RuleExpression>>,
  pub passive: Option<Vec<RuleExpression>>,
  pub onmove: Option<Vec<RuleExpression>>,
  pub aftermove: Option<Vec<RuleExpression>>,
  pub onkill: Option<Vec<RuleExpression>>, //when figure beats another
  pub possiblemoves: Vec<PMove>, //coś co można potem uruchomić na iteratorze po polach planszy by dostać listę wszystkich poprawnych pól
  pub movecondition: Option<RuleExpression>,
  pub memory: HashMap<String, RuleExpression>,

  #[serde(default)]
  pub owner: Player, // in json they will be white later board will chagne owners for them
}

impl Piece {
  pub fn new(s: String) -> Result<Self, Box<dyn Error>> {
    let piece: Self = serde_json::from_str(&s)?;
    Ok(piece)
  }

  pub fn create(s: String, id: i32) -> Result<Self, Box<dyn Error>> {
    let mut piece: Self = serde_json::from_str(&s)?;
    piece
      .memory
      .insert("id".to_string(), RuleExpression::Integer(id));
    Ok(piece)
  }

  pub fn defualt_piece() -> Self {
    Piece {
      id: "".to_string(),
      name: "".to_string(),
      deathrattle: None,
      battlecry: None,
      passive: None,
      onmove: None,
      aftermove: None,
      onkill: None,
      possiblemoves: vec![],
      movecondition: None,
      memory: HashMap::new(),
      owner: Player::White,
    }
  }

  pub fn set_owner(&mut self, pl: Player) {
    self.owner = pl;
  }

  pub fn switch_owner(&mut self) {
    self.owner = !self.owner;
  }
}

impl std::fmt::Display for Piece {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let first_char = self.name.chars().next().unwrap_or(' ');
    let formatted_char = match self.owner {
      Player::White => first_char.to_lowercase().to_string(),
      Player::Black => first_char.to_uppercase().to_string(),
    };
    write!(f, "{}", formatted_char)
  }
}
