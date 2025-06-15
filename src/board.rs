use crate::piece::Piece;
use crate::player::Player;
use crate::GameState;
use crate::PMove;
use crate::RuleExpression;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

macro_rules! evaluate_rules {
  ($board:ident, $pos:expr, $field:ident,  $context:expr, $depth:expr, $send:expr, $receive:expr, $is_evaluate_mate:expr) => {
    if let Some(piece) = &$board.board[$pos.0 as usize][$pos.1 as usize] {
      if let Some(rule) = piece.$field.clone() {
        rule.iter().for_each(|x| {
          let _ = x.evaluate($board, $context, $depth, $send, $receive, $is_evaluate_mate);
        });
      }
    }
  };
}

macro_rules! unwrap_or_return {
  ($option:expr, $err_variant:expr) => {
    match $option {
      Some(value) => value,
      None => return $err_variant,
    }
  };
}

macro_rules! unwrapres_or_return {
  ($option:expr, $err_variant:expr) => {
    match $option {
      Ok(value) => value,
      Err(_) => return $err_variant,
    }
  };
}

///plansza
#[derive(Serialize, Deserialize, Clone)]
pub struct Board {
  pub size: (u32, u32),
  pub board: Vec<Vec<Option<Piece>>>,
  pub pieces: HashMap<String, String>, // all figure types in current game
  pub cementaries: (Vec<Piece>, Vec<Piece>),
  pub endcondition: Option<RuleExpression>, // if none then end if one of them wins
  pub wincondition: (RuleExpression, RuleExpression), //if none or both evaluates to true its a draw
  #[serde(default = "Player::white")]
  pub current_player: Player,
  #[serde(default = "VecDeque::new")]
  pub history: VecDeque<(PMove, ((u32, u32), (u32, u32)))>, // move and (src dest);  this may need to be updated with other rules
  #[serde(default)] //default is 0
  pub history_size: u32,
  #[serde(default = "default_revive")]
  pub revive: RuleExpression,
}

fn default_revive() -> RuleExpression {
  RuleExpression::Boolean(false)
}

/// [name] -> (number of args, description)
pub fn default_calls() -> HashMap<String, (u32, String)> {
  HashMap::from([("is_empty".to_string(),(u32::MAX-1, "checks if given position is empty\nis_empty [[pos position]|[var position_name]|[history [src|dest] index]]\neg: is_empty var new_position".to_string())),
             ("is_attacked".to_string(),(u32::MAX-1,"checks if given position is attacked\nis_attacked [[pos position]|[var position_name]|[history [src|dest] index]]\neg: is_attacked pos 1,1".to_string())), ("is_type".to_string(),(3, "checks if piece at a given position or named cementary index has given id\nis_type [[pos position]|[var position_name]|[[cementary_white| cementary_black] idx]] [piece_id]\neg:is_type pos 1,1 pawn".to_string())), ("is_opponent".to_string(),(u32::MAX-1, "checks if given position has opponent piece\nis_opponent [[pos position]|[var position_name]|[history [src|dest] index]]\neg: is_opponent var new_position".to_string())),
             ("is_ally".to_string(),(u32::MAX-1, "checks if given position has ally piece\nis_ally [[pos position]|[var position_name]|[history [src|dest] index]]\neg: is_ally var new_position".to_string())), ("is_different".to_string(),(2, "checks if two named positions are different\nis_different [position1_name] [position2_name]\neg: is_different new_position old_position".to_string())), ("in_bounds".to_string(),(u32::MAX-1,"checks if given position is in board bounds\nin_bounds [[pos position]|[var position_name]|[history [src|dest] index]]\neg: in_bounds pos 1,1".to_string())),
             ("is_path_blocked".to_string(),(2, "checks if path between two named positions is blocked, when the positions are not in straight line or square diagonal returns false\nis_path_blocked [position1_name] [position2_name]\neg: is_path_blocked old_position new_position".to_string())), ("get_vertical_cord".to_string(),(0, "returns vertical coordinate of old_position\nget_vertical_cord\neg: get_vertical_cord".to_string())), ("get_horizontal_cord".to_string(),(0,"returns horizontal coordinate of old_position\nget_horizontal_cord\neg: get_horizontal_cord".to_string())),
             ("get_target_vertical_cord".to_string(),(0,"returns vertical coordinate of new_position\nget_target_vertical_cord\neg: get_target_vertical_cord".to_string())), ("get_target_horizontal_cord".to_string(),(0,"returns horizontal coordinate of new_position\nget_target_horizontal_cord\neg: get_target_horizontal_cord".to_string())), ("get_memory".to_string(), (3,"returns memory of piece with given id/on given position\nget_memory [[pos position]|[var position_name]|[id piece_memory_id]|[[cementary_white| cementary_black] variable_name]] [memory_name]\neg: get_memory var old_position move_count".to_string())),("increase_memory".to_string(),(5,"increase given memory of a piece with given id/on given position\nincrease_memory [pair|int] [[pos position]|[var position_name]|[id piece_memory_id]] [memory_name] [value]\neg: increase_memory int var new_position move_count 13".to_string())),("increase_var".to_string(),(3,"increase given context variable\nincrease_var [pair|int] [variable_name] value\neg: increase_var pair special_position 1,1".to_string())),
             ("change_type".to_string(), (u32::MAX, "change type of piece at new_position to one of listed ones\nchange_type [piece_id1] ...\neg: change_type pawn rook".to_string())), ("forward".to_string(), (u32::MAX-1, "returns vertical difference relative to pawn at 'old_position'\nforward [[position|variable] value ]| [history [src|dest] index]\neg: forward variable new_position".to_string())),("left".to_string(), (u32::MAX-1, "returns horizontal difference relative to pawn at 'old_position'\nleft [[position|variable] value ]| [history [src|dest] index]\neg: left variable new_position".to_string())), ("north".to_string(), (u32::MAX-1, "returns vertical difference to 'old_position'\nnorth [[position|variable] value ]| [history [src|dest] index]\neg: north variable new_position".to_string())),("west".to_string(), (u32::MAX-1, "returns horizontal difference to 'old_position'\nwest [[position|variable] value ]| [history [src|dest] index]\neg: west variable new_position".to_string())),("history".to_string(), (u32::MAX-1, "returns move from history\nhistory [src|dest] [idx] (v|h)\neg: history dest 0 v".to_string())), ("kill".to_string(),(u32::MAX-1 ,"kills piece at specified position\nkill [[position|variable] value ]| [history [src|dest] index]\neg: kill history dest 0".to_string())), ("piece_on_board".to_string(), (1, "checks if there is at least one piece with given id on board\npiece_on_board [piece_id]\neg: piece_on_board pawn".to_string())), ("piece_on_board_cnt".to_string(),(1,"returns number of pieces with given id on board\npiece_on_board_cnt [piece_id]\neg: piece_on_board_cnt pawn".to_string())), ("player_piece_on_board".to_string(), (2, "checks if given player has at least one piece of given type on board\nplayer_piece_on_board [piece_id] [player]\neg: player_piece_on_board pawn White".to_string())), ("player_piece_on_board_cnt".to_string(), (2, "returns number of pieces of given type given player has on board\nplayer_piece_on_board_cnt [piece_id] [player]\neg: player_piece_on_board_cnt pawn Black".to_string())),("move_piece".to_string(),(3, "moves piece at src to dst\nmove_piece [var|pos] src dst\neg: move_piece pos 1,1 2,1".to_string())), ("is_same_line".to_string(), (3, "checks if all position on give line are the same, returns error when positions are not in the same line or square diagonal\nis_same_line [position1] [position2] [piece_id|owner]\neg: is_same_line 6,6 7,7 owner".to_string())), ("check_mate".to_string(), (2, "checks if piece with given id in memory is mated by given player\ncheck_mate [piece_id] [player]\n eg: check_mate 1 White".to_string())),("create_var_from_mem".to_string(), (4, "creates context variable from piece memory\ncreate_var_from_mem [[pos position]|[id piece_id ]| [var position_name]] [memory_name] [variable_name]\neg: create_var_from_mem pos 1,1 move_count var1".to_string())),("create_var_from_history".to_string(),(4,"createst context variable from history entry\ncreate_var_from_history [src|dst] [vertical|horizontal|pair] [idx] [variable_name]\neg: create_var_from_history src pair 0 var1".to_string())), ("clone".to_string(),(2, "creates a copy of context variable\nclone [variable_name1] [variable_name2]\neg: clone new_position position2".to_string())), ("is_player".to_string(), (2, "checks if given position has figure belonging to given player\nis_player [variable_name] [player]\neg:is_player old_position White".to_string()))
                  ])
}

impl Board {
  ///tworzy planszę na podstawie pliku json
  pub fn from_json(s: String) -> Result<Self, Box<dyn Error>> {
    /*
    if we want to have id's
        we can have them predefined in json then we have to change initialboard to just hold options of piece instead of pair figure name and owner. or hold triples of figure piece and id'

        or we can do that on init, then better not to touch the json.

        for now i go with triples
    */

    let mut board: Board = serde_json::from_str(&s)?;

    let parsed: Value = serde_json::from_str(&s)?;

    let inib = serde_json::from_value(parsed["initialboard"].clone());
    let icement = serde_json::from_value(parsed["initialcementaries"].clone());
    if inib.is_ok() {
      let brd: Vec<Vec<Option<(String, String, i32)>>> = inib.unwrap();

      board.board = brd
        .iter()
        .map(|x| {
          x.iter()
            .map(|y| {
              if let Some((p1, p2, p3)) = y {
                if !board.pieces.contains_key(p1) {
                  return None; //incorrect json
                }
                let mut piece = Piece::new(board.pieces[p1].clone()).ok().unwrap(); //assume jsons for figures are ok for now

                piece.set_owner(unwrap_or_return!(
                  Player::from_str(p2),
                  None //incorrect json
                ));

                piece
                  .memory
                  .insert("id".to_string(), RuleExpression::Integer(*p3));

                Some(piece)
              } else {
                None
              }
            })
            .collect()
        })
        .collect();
    }

    if icement.is_ok() {
      let cmt: Vec<Option<(String, String, i32)>> = icement.unwrap();
      board.cementaries.0 = Vec::new();
      board.cementaries.1 = Vec::new();

      for x in cmt.into_iter() {
        if let Some((p1, p2, p3)) = x {
          if !board.pieces.contains_key(&p1) {
            return Err(format!("no {} defined in board", p1).into());
          }
          let mut piece = Piece::new(board.pieces[&p1].clone()).ok().unwrap();

          piece
            .memory
            .insert("id".to_string(), RuleExpression::Integer(p3));

          let own = Player::from_str(&p2);

          if own.is_none() {
            return Err("cannot set owner".to_string().into());
          }

          piece.owner = own.unwrap();

          if piece.owner == Player::White {
            board.cementaries.0.push(piece);
          } else {
            board.cementaries.1.push(piece);
          }
        }
      }
    }

    if !board.check_ids() {
      return Err("ids not distinct".into());
    }
    Ok(board)
  }

  ///tworzy domyślną, pustą planszę
  pub fn new() -> Board {
    let mut board = Vec::with_capacity(8);
    for _i in 0..8 {
      let mut row = Vec::with_capacity(8);
      for _j in 0..8 {
        row.push(None);
      }
      board.push(row);
    }
    Board {
      size: (8, 8),
      board,
      pieces: HashMap::new(),
      cementaries: (Vec::new(), Vec::new()),
      endcondition: Some(RuleExpression::Boolean(false)),
      wincondition: (
        RuleExpression::Boolean(false),
        RuleExpression::Boolean(false),
      ),
      history: VecDeque::new(),
      history_size: 1,
      current_player: Player::White,
      revive: RuleExpression::Boolean(false),
    }
  }

  pub fn check_ids(&self) -> bool {
    let mut ids: Vec<i32> = Vec::new();
    for p in &self.cementaries.0 {
      match p.memory.get("id") {
        Some(RuleExpression::Integer(id)) => ids.push(*id),
        _ => return false,
      }
    }
    for p in &self.cementaries.1 {
      match p.memory.get("id") {
        Some(RuleExpression::Integer(id)) => ids.push(*id),
        _ => return false,
      }
    }

    for x in 0..self.size.0 {
      for y in 0..self.size.1 {
        match &self.board[x as usize][y as usize] {
          Some(p) => match p.memory.get("id") {
            Some(RuleExpression::Integer(id)) => ids.push(*id),
            _ => return false,
          },
          None => {}
        }
      }
    }
    ids.sort();
    ids.windows(2).all(|w| w[0] != w[1])
  }

  pub fn add_piece(&mut self, json: String) -> Result<(), String> {
    match serde_json::from_str::<Value>(&json) {
      Ok(data) => {
        if let Some(id) = data["id"].as_str() {
          self.pieces.insert(id.to_string(), json);
          Ok(())
        } else {
          Err("this shouldn't be possible".to_string())
        }
      }
      Err(e) => Err(e.to_string()),
    }
  }

  pub fn add_piece2(&mut self, piece: Piece) -> Result<(), String> {
    if self.pieces.contains_key(&piece.id) {
      Err("piece already on board".to_string())
    } else {
      let pom = serde_json::to_string(&piece);
      if pom.is_err() {
        Err("something when wrong with to_sting on this piece".to_string())
      } else {
        self.pieces.insert(piece.id, pom.unwrap());
        Ok(())
      }
    }
  }

  ///dodaj ruch do historii
  pub fn add_history(&mut self, pmove: PMove, from: (u32, u32), to: (u32, u32)) {
    if self.history_size == 0 {
      return ();
    }

    if self.history.len() == self.history_size as usize {
      self.history.pop_back();
    }

    self.history.push_front((pmove, (from, to)))
  }

  pub fn get_history(&mut self, idx: u32) -> Option<(PMove, ((u32, u32), (u32, u32)))> {
    if idx >= self.history_size || (idx as usize) >= self.history.len() {
      return None;
    }
    Some(self.history[idx as usize].clone())
  }

  ///sprawdź, czy linia od pola do pola jest blokowana przez coś
  pub fn is_path_blocked(&self, from: (u32, u32), to: (u32, u32)) -> bool {
    let (fx, fy) = from;
    let (tx, ty) = to;

    let dx = (tx as i32 - fx as i32).signum();
    let dy = (ty as i32 - fy as i32).signum();

    if dx == 0 && dy == 0 {
      return false;
    }

    if dx != 0 && dy != 0 && (tx as i32 - fx as i32).abs() != (ty as i32 - fy as i32).abs() {
      return false;
    }

    let mut x = fx as i32;
    let mut y = fy as i32;

    loop {
      x = x + dx;
      y = y + dy;

      if (x, y) == (tx as i32, ty as i32) {
        break;
      }

      if x < 0 || y < 0 || x >= self.size.0 as i32 || y >= self.size.1 as i32 {
        break;
      }
      if self.board[x as usize][y as usize].is_some() {
        return true;
      }
    }

    false
  }

  ///sprawdza, czy istnieje figura na danej pozycji
  pub fn has_piece_at(&self, position: (u32, u32)) -> bool {
    let (x, y) = position;
    self.board[x as usize][y as usize].is_some()
  }

  ///parsuje i sprawdza, czy pozycja jest w obrębie planszy
  pub fn is_position_valid(&self, pos_str: &str) -> bool {
    if let Some((x, y)) = self.parse_position(pos_str) {
      return x < self.size.0 && y < self.size.1;
    }
    false
  }

  ///parsuje pozycję zapisaną jako string na (u32, u32)
  pub fn parse_position(&self, pos_str: &str) -> Option<(u32, u32)> {
    //"1, 2"
    let parts: Vec<&str> = pos_str.split(',').collect();
    if parts.len() == 2 {
      let x = parts[0].parse();
      let y = parts[1].parse();
      if x.is_ok() && y.is_ok() {
        return Some((x.unwrap(), y.unwrap()));
      }
      //Some((x, y))
    }
    None
  }

  ///parsuje pozycję zapisaną jako string na (i32, i32)
  pub fn parse_pair(&self, pos_str: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = pos_str.split(',').collect();
    if parts.len() == 2 {
      let x = parts[0].parse();
      let y = parts[1].parse();
      if x.is_ok() && y.is_ok() {
        return Some((x.unwrap(), y.unwrap()));
      }
    }
    None
  }

  ///lista wszystkich pozycji osiągalnych dla figury z pozycji
  pub fn get_possible_positions(
    &mut self,
    old_position: (u32, u32),
    depth: u32,
  ) -> Option<Vec<(u32, u32)>> {
    let (x, y) = old_position;
    if x >= self.size.0 || y >= self.size.1 {
      return None;
    }
    //                            V perfectly safe, as checked above, just don't want to deal with error handling
    let piece: &Piece = match unsafe {
      self
        .board
        .get_unchecked(old_position.0 as usize)
        .get_unchecked(old_position.1 as usize)
        .as_ref()
    } {
      Some(f) => f,
      None => return None,
    };
    let mut ret = Vec::new();
    let possible_moves = piece.possiblemoves.clone();
    let movecondition = piece.movecondition.clone();
    let mut context = HashMap::new(); // TODO maybe adding context as argument, for now not needed
    context.insert(
      String::from("old_position"),
      format!("{},{}", x, y).to_string(),
    );

    for x in 0..self.size.0 {
      for y in 0..self.size.1 {
        context.insert(
          String::from("new_position"),
          format!("{},{}", x, y).to_string(),
        );
        // TODO later it should return pairs of positions and pmove.

        let move_possible = possible_moves.iter().any(|pmove| {
          matches!(
            pmove
              .condition
              .evaluate(self, &mut context, depth, None, None, false),
            RuleExpression::Boolean(true)
          )
        });

        let condition_met = movecondition.as_ref().map_or(true, |condition| {
          matches!(
            condition.evaluate(self, &mut context, depth, None, None, false),
            RuleExpression::Boolean(true)
          )
        });

        if move_possible && condition_met {
          ret.push((x, y));
        }
      }
    }

    Some(ret)
  }

  ///lista wszystkich ruchów figury z pozycji
  pub fn get_possible_moves(
    &mut self,
    old_position: (u32, u32),
    depth: u32,
  ) -> Option<Vec<((u32, u32), PMove)>> {
    let (x, y) = old_position;
    if x >= self.size.0 || y >= self.size.1 {
      return None;
    }
    //                            V perfectly safe, as checked above, just don't want to deal with error handling
    let piece: &Piece = match unsafe {
      self
        .board
        .get_unchecked(old_position.0 as usize)
        .get_unchecked(old_position.1 as usize)
        .as_ref()
    } {
      Some(f) => f,
      None => return None,
    };
    let mut ret = Vec::new();
    let possible_moves = piece.possiblemoves.clone();
    let movecondition = piece.movecondition.clone();
    let mut context = HashMap::new(); // TODO maybe adding context as argument, for now not needed

    if movecondition.as_ref().is_some_and(|v| {
      v.evaluate(self, &mut context, depth, None, None, false) == RuleExpression::Boolean(false)
    }) {
      return None;
    }

    context.insert(
      String::from("old_position"),
      format!("{},{}", x, y).to_string(),
    );

    for x in 0..self.size.0 {
      for y in 0..self.size.1 {
        context.insert(
          String::from("new_position"),
          format!("{},{}", x, y).to_string(),
        );

        let move_possible = possible_moves
          .iter()
          .filter(|pmove| {
            matches!(
              pmove
                .condition
                .evaluate(self, &mut context, depth, None, None, false),
              RuleExpression::Boolean(true)
            )
          })
          .map(|v| ((x, y), v.clone()));

        ret.extend(move_possible);
      }
    }

    if ret.is_empty() {
      return None;
    }
    Some(ret)
  }

  ///lista ruchów z a do b
  pub fn get_moves_to(
    &mut self,
    old_position: (u32, u32),
    new_position: (u32, u32),
    depth: u32,
  ) -> Option<Vec<PMove>> {
    let (x, y) = old_position;
    let (x2, y2) = new_position;
    if x >= self.size.0 || y >= self.size.1 || x2 >= self.size.0 || y2 >= self.size.1 {
      return None;
    }
    //                            V perfectly safe, as checked above, just don't want to deal with error handling
    let piece: &Piece = match unsafe {
      self
        .board
        .get_unchecked(old_position.0 as usize)
        .get_unchecked(old_position.1 as usize)
        .as_ref()
    } {
      Some(f) => f,
      None => return None,
    };
    let mut ret = Vec::new();
    let possible_moves = piece.possiblemoves.clone();
    let movecondition = piece.movecondition.clone();
    let mut context = HashMap::new(); // TODO maybe adding context as argument, for now not needed

    context.insert(
      String::from("old_position"),
      format!("{},{}", x, y).to_string(),
    );
    context.insert(
      String::from("new_position"),
      format!("{},{}", x2, y2).to_string(),
    );

    if movecondition.as_ref().is_some_and(|v| {
      v.evaluate(self, &mut context, depth, None, None, false) == RuleExpression::Boolean(false)
    }) {
      return None;
    }

    let move_possible = possible_moves
      .iter()
      .filter(|pmove| {
        matches!(
          pmove
            .condition
            .evaluate(self, &mut context, depth, None, None, false),
          RuleExpression::Boolean(true)
        )
      })
      .map(|v| v.clone());

    ret.extend(move_possible);

    if ret.is_empty() {
      return None;
    }

    Some(ret)
  }

  pub fn revive_piece(
    &mut self,
    white: bool,
    idx: u32,
    position: (u32, u32),
    sender: &Sender<Option<String>>,
    receiver: &Receiver<String>,
    is_evaluate_mate: bool,
  ) -> Result<GameState, String> {
    let mut context = HashMap::new();
    context.insert(
      String::from("new_position"),
      format!("{},{}", position.0, position.1).to_string(),
    );
    context.insert(String::from("revive_index"), format!("{}", idx).to_string());
    context.insert(
      String::from("player_revived"),
      format!("{}", if white { "1" } else { "0" }),
    );

    let mut board_clone = self.clone();
    if self.revive.clone().evaluate(
      &mut board_clone,
      &mut context,
      0,
      Some(sender),
      Some(receiver),
      is_evaluate_mate,
    ) != RuleExpression::Boolean(true)
    {
      return Err("Cannot revive".to_string());
    }

    let piece;
    if white {
      if self.cementaries.0.len() <= idx as usize {
        return Err("no piece at that cementary index".to_string());
      }
      piece = self.cementaries.0[idx as usize].clone();
      self.cementaries.0.swap_remove(idx as usize);
    } else {
      if self.cementaries.1.len() <= idx as usize {
        return Err("no piece at that cementary index".to_string());
      }
      piece = self.cementaries.1[idx as usize].clone();
      self.cementaries.1.swap_remove(idx as usize);
    }

    let mut kill = false;
    if let Some(ref piece3) = self.board[position.0 as usize][position.1 as usize] {
      let mut piece2 = piece3.clone();
      piece2.owner = !piece3.owner;
      match piece2.owner {
        Player::White => self.cementaries.0.push(piece2.clone()), // white pieces killed, cemetery
        Player::Black => self.cementaries.1.push(piece2.clone()), // Black pieces killed, cemetery
      }
      kill = true;
    } else {
      // leave it for now since we might want to do actions only if not kill
    }
    self.board[position.0 as usize][position.1 as usize] = Some(piece);

    evaluate_rules!(
      self,
      position,
      battlecry,
      &mut context,
      0,
      Some(sender),
      Some(receiver),
      is_evaluate_mate
    );

    if kill {
      evaluate_rules!(
        self,
        position,
        deathrattle,
        &mut context,
        0,
        Some(sender),
        Some(receiver),
        is_evaluate_mate
      );
    }

    return self.get_game_state(&mut context);
  }
  ///returns the list of not current_player positions that attack a given position,
  pub fn get_attackers_player(
    &mut self,
    position: (u32, u32),
    pp: Player,
    depth: u32,
  ) -> Option<Vec<(u32, u32)>> {
    //assumes to get a valid position

    let mut attackers: Vec<(u32, u32)> = vec![];
    for x in 0..self.size.0 {
      for y in 0..self.size.1 {
        if (x, y) != position
          && self.board[x as usize][y as usize]
            .as_ref()
            .is_some_and(|v| v.owner != pp)
          && self.get_moves_to((x, y), position, depth).is_some()
        {
          attackers.push((x, y));
        }
      }
    }
    if attackers.len() != 0 {
      return Some(attackers);
    }
    None
  }

  ///returns true if position is attacked, just to avoid always checking if is some
  pub fn is_attacked_player(&mut self, position: (u32, u32), opponent: bool, depth: u32) -> bool {
    let pp = if opponent {
      self.current_player
    } else {
      !self.current_player
    };
    let old = self.current_player;
    self.current_player = !pp;
    let a = self.get_attackers_player(position, pp, depth).is_some();
    self.current_player = old;
    a
  }

  ///returns true if piece at position is attacked, just to avoid always checking if is some
  pub fn is_attacked_piece(&mut self, position: (u32, u32), opponent: bool, depth: u32) -> bool {
    if self.board[position.0 as usize][position.1 as usize].is_none() {
      return false;
    }
    let old = self.current_player;
    let mut pp = unwrap_or_return!(
      self.board[position.0 as usize][position.1 as usize].clone(),
      false
    )
    .owner;
    if !opponent {
      pp = !pp;
    }
    self.current_player = !pp;
    let a = self.get_attackers_player(position, pp, depth).is_some();
    self.current_player = old;
    a
  }

  ///returns the list of position that can attack given position
  pub fn get_attackers(&mut self, position: (u32, u32), depth: u32) -> Option<Vec<(u32, u32)>> {
    //assumes to get a valid position
    let mut attackers: Vec<(u32, u32)> = vec![];
    for x in 0..self.size.0 {
      for y in 0..self.size.1 {
        if (x, y) != position && self.get_moves_to((x, y), position, depth).is_some() {
          attackers.push((x, y));
        }
      }
    }
    if attackers.len() != 0 {
      return Some(attackers);
    }
    None
  }

  ///returns true if position is attacked, just to avoid always checking if is some
  pub fn is_attacked(&mut self, position: (u32, u32), depth: u32) -> bool {
    self.get_attackers(position, depth).is_some()
  }

  ///returns positions that piece at a given position attacks
  pub fn get_attacked(&mut self, position: (u32, u32), depth: u32) -> Option<Vec<(u32, u32)>> {
    // for now i assume that pieces know whether they can kill ally pieces
    Some(
      unwrap_or_return!(self.get_possible_positions(position, depth), None)
        .into_iter()
        .filter(|(x, y)| self.board[*x as usize][*y as usize].is_some())
        .collect(),
    )
  }

  ///returns true if piece at given position can attack
  pub fn can_attack(&mut self, position: (u32, u32), depth: u32) -> bool {
    self.get_attacked(position, depth).is_some()
  }

  ///returns positions that piece at a given position attacks
  pub fn get_attacked_player(
    &mut self,
    position: (u32, u32),
    opponent: bool,
    depth: u32,
  ) -> Option<Vec<(u32, u32)>> {
    if self.board[position.0 as usize][position.1 as usize].is_none() {
      return None;
    }

    let pp: Player = if opponent {
      !(self.board[position.0 as usize][position.1 as usize]
        .as_ref()
        .unwrap()
        .owner)
    } else {
      self.board[position.0 as usize][position.1 as usize]
        .as_ref()
        .unwrap()
        .owner
    };

    Some(
      unwrap_or_return!(self.get_possible_positions(position, depth), None)
        .into_iter()
        .filter(|(x, y)| {
          self.board[*x as usize][*y as usize]
            .as_ref()
            .is_some_and(|v| v.owner != pp)
        })
        .collect(),
    )
  }

  ///returns true if piece at given position can attack
  pub fn can_attack_player(&mut self, position: (u32, u32), opponent: bool, depth: u32) -> bool {
    self
      .get_attacked_player(position, opponent, depth)
      .is_some()
  }

  ///returns value of variable for piece at given position
  pub fn get_piece_var(
    &mut self,
    position: (u32, u32),
    variable: &str,
    context: &mut HashMap<String, String>,
    depth: u32,
    sender: Option<&Sender<Option<String>>>,
    receiver: Option<&Receiver<String>>,
    is_evaluate_mate: bool,
  ) -> RuleExpression {
    if let Some(ref mut piece) = self.board[position.0 as usize][position.1 as usize] {
      match &mut piece.memory.get(variable) {
        Some(val) => {
          return val.clone().evaluate(
            self,
            context,
            depth + 1,
            sender,
            receiver,
            is_evaluate_mate,
          );
        }
        None => {
          return RuleExpression::Err(format!(
            "piece {} does not have {} variable in {}",
            piece.name, variable, "get_piece_var func"
          ));
        }
      }
    } else {
      return RuleExpression::Err(format!(
        "no piece at given position in {}",
        "get_piece_var func"
      ));
    }
  }

  pub fn match_and_get_valid_position(
    &mut self,
    args: Vec<&str>,
    context: &mut HashMap<String, String>,
  ) -> Option<(u32, u32)> {
    match args[0] {
      "position" | "pos" | "p" => self.get_valid_position(args[1]),
      "variable" | "var" | "v" => {
        if context.contains_key(args[1]) {
          self.get_valid_position(&context[args[1]])
        } else {
          None
        }
      }
      // dla history jest history src|dest number
      "history" | "h" => {
        let pomi = unwrap_or_return!(
          self.get_history(unwrapres_or_return!(args[2].parse::<u32>(), None)),
          None
        );

        match args[1] {
          "src" | "source" => Some(pomi.1 .0),
          "dst" | "dest" | "destination" => Some(pomi.1 .1),
          _ => {
            return None;
          }
        }
      }

      _ => None,
    }
  }

  pub fn match_and_get_pair(
    &mut self,
    args: Vec<&str>,
    context: &mut HashMap<String, String>,
  ) -> Option<(i32, i32)> {
    match args[0] {
      "position" | "pos" | "p" => self.get_pair(args[1]),
      "variable" | "var" | "v" => {
        if context.contains_key(args[1]) {
          self.get_pair(&context[args[1]])
        } else {
          None
        }
      }
      // dla history jest history src|dest number
      "history" | "h" => {
        let pomi = unwrap_or_return!(
          self.get_history(unwrapres_or_return!(args[2].parse::<u32>(), None)),
          None
        );

        match args[1] {
          "src" | "source" => Some((pomi.1 .0 .0 as i32, pomi.1 .0 .1 as i32)),
          "dst" | "dest" | "destination" => Some((pomi.1 .1 .0 as i32, pomi.1 .1 .1 as i32)),
          _ => {
            return None;
          }
        }
      }

      _ => None,
    }
  }
  ///uproszczenie zapytania api do wartości
  pub fn call_api(
    &mut self,
    api: &str,
    args: Vec<&str>,
    context: &mut HashMap<String, String>,
    depth: u32,
    sender: Option<&Sender<Option<String>>>,
    receiver: Option<&Receiver<String>>,
    is_evaluate_mate: bool,
  ) -> RuleExpression {
    // println!("{:?}, {:?}",api, args);
    match api {
      "is_empty" => {
        //is_empty [[pos position]|[var position_name]|[history src|dest index]]

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.match_and_get_valid_position(args, context),
          RuleExpression::Err(String::from("could not get a valid position"))
        );
        RuleExpression::Boolean(!self.has_piece_at(pos))
      }

      "is_attacked" => {
        //is_attacked [[pos position]|[var position_name]|[history src|dest index]]
        // TODO add changes when you can kill your own pieces

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.match_and_get_valid_position(args, context),
          RuleExpression::Err(String::from("could not get a valid position"))
        );

        let x = RuleExpression::Boolean(self.is_attacked_player(pos, true, depth));
        x
      }

      "is_type" => {
        //is_type [[pos position]|[var position_name]|[[cementary_white| cementary_black] variable_name]] [piece_id]
        if args.len() != 3 {
          return RuleExpression::Err("wrong number of arguments in is_type".to_string());
        }

        let pos = match args[0] {
          "pos" | "position" => {
            unwrap_or_return!(
              self.get_valid_position(args[1]),
              RuleExpression::Err(String::from("not a valid position"))
            )
          }
          "var" | "variable" => {
            unwrap_or_return!(
              self.get_valid_position(unwrap_or_return!(
                context.get(args[1]),
                RuleExpression::Err(format!("no {} in {}", args[1], api))
              )),
              RuleExpression::Err(format!("{} is not a position {}", args[1], api))
            )
          }
          "cw" | "cementary_white" => {
            let idx = unwrapres_or_return!(
              unwrap_or_return!(
                context.get(args[1]),
                RuleExpression::Err(format!("no {} in {}", args[1], api))
              )
              .parse::<u32>(),
              RuleExpression::Err(String::from("Not a natural number"))
            );
            if idx as usize >= self.cementaries.0.len() {
              return RuleExpression::Err(String::from("index, out of cementary"));
            }
            return RuleExpression::Boolean(self.cementaries.0[idx as usize].clone().id == args[2]);
          }
          "cb" | "cementary_black" => {
            let idx = unwrapres_or_return!(
              unwrap_or_return!(
                context.get(args[1]),
                RuleExpression::Err(format!("no {} in {}", args[1], api))
              )
              .parse::<u32>(),
              RuleExpression::Err(String::from("Not a natural number"))
            );
            if idx as usize >= self.cementaries.1.len() {
              return RuleExpression::Err(String::from("index, out of cementary"));
            }
            return RuleExpression::Boolean(self.cementaries.1[idx as usize].clone().id == args[2]);
          }
          _ => {
            return RuleExpression::Err("wrong first argument".to_string());
          }
        };

        RuleExpression::Boolean(
          unwrap_or_return!(
            self.board[pos.0 as usize][pos.1 as usize].clone(),
            RuleExpression::Err("no piece at given position".to_string())
          )
          .id
            == args[2],
        )
      }

      "is_opponent" => {
        //is_opponent [[pos position]|[var position_name]|[history src|dest index]]

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.match_and_get_valid_position(args, context),
          RuleExpression::Err(String::from("could not get a valid position"))
        );

        if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {
          return RuleExpression::Boolean(piece.owner != self.current_player);
        }

        RuleExpression::Boolean(false)
      }

      "is_ally" => {
        //is_ally [[pos position]|[var position_name]|[history src|dest index]]

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.match_and_get_valid_position(args, context),
          RuleExpression::Err(String::from("could not get a valid position"))
        );

        if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {
          return RuleExpression::Boolean(piece.owner == self.current_player);
        }

        RuleExpression::Boolean(false)
      }

      "is_player" => {
        if args.len() != 2 {
          return RuleExpression::Err("wrong number of args".to_string());
        }
        let pos2 = self.get_valid_position(args[0]);
        let pos;
        if pos2.is_some() {
          pos = pos2.unwrap();
        } else {
          pos = unwrap_or_return!(
            self.get_valid_position(unwrap_or_return!(
              context.get(args[0]),
              RuleExpression::Err(format!("no {} in {}", args[0], api))
            )),
            RuleExpression::Err(format!("Could not get a valid position in {}", api))
          );
        }

        if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {
          return match args[1] {
            "White" => RuleExpression::Boolean(piece.owner == Player::White),
            "Black" => RuleExpression::Boolean(piece.owner == Player::Black),
            _ => RuleExpression::Boolean(false),
          };
        }

        RuleExpression::Boolean(false)
      }

      "is_different" => RuleExpression::Boolean(
        self.get_pair(unwrap_or_return!(
          context.get(args[0]),
          RuleExpression::Err(format!("no {} in {}", args[0], api))
        )) != self.get_pair(unwrap_or_return!(
          context.get(args[1]),
          RuleExpression::Err(format!("no {} in {}", args[1], api))
        )),
      ),

      "in_bounds" => {
        //in_bounds [[pos position]|[var position_name]|[history src|dest index]]

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos: String = match args[0] {
          "position" | "pos" | "p" => args[1].to_string(),
          "variable" | "var" | "v" => {
            if context.contains_key(args[1]) {
              context.get(args[1]).unwrap().to_string()
            } else {
              return RuleExpression::Err("given variable not in context".to_string());
            }
          }
          // dla history jest history src|dest number
          "history" | "h" => {
            let pomi = unwrap_or_return!(
              self.get_history(unwrapres_or_return!(
                args[2].parse::<u32>(),
                RuleExpression::Err("wrong third argument".to_string())
              )),
              RuleExpression::Err("could not get history entry".to_string())
            );

            let r = match args[1] {
              "src" | "source" => {
                format!("{},{}", pomi.1 .0 .0, pomi.1 .0 .1)
              }
              "dst" | "dest" | "destination" => {
                format!("{},{}", pomi.1 .1 .0, pomi.1 .1 .1)
              }
              _ => {
                return RuleExpression::Err("wrong second argument".to_string());
              }
            };

            r
          }

          _ => {
            return RuleExpression::Err("wrong first argument".to_string());
          }
        };

        RuleExpression::Boolean(self.is_position_valid(pos.as_str()))
      }

      "is_path_blocked" => {
        if args.len() != 2 {
          return RuleExpression::Err("wrong number of argument".to_string());
        }
        let c1 = unwrap_or_return!(
          context.get(args[0]),
          RuleExpression::Err(format!("no {} in {}", args[0], api))
        );
        let c2 = unwrap_or_return!(
          context.get(args[1]),
          RuleExpression::Err(format!("no {} in {}", args[1], api))
        );
        RuleExpression::Boolean(self.is_path_blocked(
          unwrap_or_return!(
            self.get_valid_position(c1),
            RuleExpression::Err(format!("old_position in {}", api))
          ),
          unwrap_or_return!(
            self.get_valid_position(c2),
            RuleExpression::Err(format!("new_position in {}", api))
          ),
        ))
      }

      "get_vertical_cord" => RuleExpression::Integer(
        unwrap_or_return!(
          self.get_pair(unwrap_or_return!(
            context.get("old_position"),
            RuleExpression::Err(format!("no old_position in {}", api))
          )),
          RuleExpression::Err(format!("old_position in {}", api))
        )
        .0 as i32,
      ),

      "get_horizontal_cord" => RuleExpression::Integer(
        unwrap_or_return!(
          self.get_pair(unwrap_or_return!(
            context.get("old_position"),
            RuleExpression::Err(format!("no old_position in {}", api))
          )),
          RuleExpression::Err(format!("old_position in {}", api))
        )
        .1 as i32,
      ),

      "get_target_vertical_cord" => {
        let pos = unwrap_or_return!(
          self.get_pair(unwrap_or_return!(
            context.get("new_position"),
            RuleExpression::Err(format!("no new_position in {}", api))
          )),
          RuleExpression::Err(format!("new_position in {}", api))
        );
        // println!("get_target_vertical_cord got {:?}", pos);
        RuleExpression::Integer(pos.0 as i32)
      }

      "get_target_horizontal_cord" => {
        let pos = unwrap_or_return!(
          self.get_pair(unwrap_or_return!(
            context.get("new_position"),
            RuleExpression::Err(format!("no new_position in {}", api))
          )),
          RuleExpression::Err(format!("new_position in {}", api))
        );
        RuleExpression::Integer(pos.1 as i32)
      }
      "get_memory" => {
        //get_memory [[pos position]|[var position_name]|[id piece_memory_id]|[[cementary_white| cementary_black] variable_name]] [memory_name]
        if args.len() != 3 {
          return RuleExpression::Err("wrong number of arguments".to_string());
        }
        let pos: (u32, u32) = match args[0] {
          "id" => {
            unwrap_or_return!(
              self.find_piece_creation_id(unwrapres_or_return!(
                args[0].parse::<i32>(),
                RuleExpression::Err(format!("Not a valid id in {}", api))
              )),
              RuleExpression::Err("could not find piece with given id on board".to_string())
            )
          }
          "pos" | "position" => {
            unwrap_or_return!(
              self.get_valid_position(args[1]),
              RuleExpression::Err(String::from("not a valid position"))
            )
          }
          "var" | "variable" => {
            unwrap_or_return!(
              self.get_valid_position(unwrap_or_return!(
                context.get(args[1]),
                RuleExpression::Err(format!("no {} in {}", args[1], api))
              )),
              RuleExpression::Err(format!("{} is not a position {}", args[1], api))
            )
          }
          "cw" | "cementary_white" => {
            let idx = unwrapres_or_return!(
              unwrap_or_return!(
                context.get(args[1]),
                RuleExpression::Err(format!("no {} in {}", args[1], api))
              )
              .parse::<u32>(),
              RuleExpression::Err(String::from("Not a natural number"))
            );
            if idx as usize >= self.cementaries.0.len() {
              return RuleExpression::Err(String::from("index, out of cementary"));
            }
            return unwrap_or_return!(
              self.cementaries.0[idx as usize].clone().memory.get(args[2]),
              RuleExpression::Err(String::from("Piece does not have given memory"))
            )
            .clone()
            .evaluate(self, context, depth + 1, sender, receiver, is_evaluate_mate);
            //  return RuleExpression::Boolean(self.cementaries.0[idx as usize].clone().id==args[2]);
          }
          "cb" | "cementary_black" => {
            let idx = unwrapres_or_return!(
              unwrap_or_return!(
                context.get(args[1]),
                RuleExpression::Err(format!("no {} in {}", args[1], api))
              )
              .parse::<u32>(),
              RuleExpression::Err(String::from("Not a natural number"))
            );
            if idx as usize >= self.cementaries.1.len() {
              return RuleExpression::Err(String::from("index, out of cementary"));
            }
            return unwrap_or_return!(
              self.cementaries.1[idx as usize].clone().memory.get(args[2]),
              RuleExpression::Err(String::from("Piece does not have given memory"))
            )
            .clone()
            .evaluate(self, context, depth + 1, sender, receiver, is_evaluate_mate);
          }
          _ => {
            return RuleExpression::Err("wrong first argument".to_string());
          }
        };
        let expr = self.get_piece_var(
          pos,
          args[2],
          context,
          depth,
          sender,
          receiver,
          is_evaluate_mate,
        );
        return expr;
        // println!("{:?}", expr);
        // expr.evaluate(self, context, depth+1, sender, receiver, is_evaluate_mate) //potenctially will cause problems since it evaluates expression twice
      }

      "increase_memory" => {
        // increase_memory [pair|int] [[pos position]|[var position_name]|[id piece_memory_id]] [memory_name] [value]
        if args.len() != 5 {
          return RuleExpression::Err("wrong number of arguments".to_string());
        }
        let pos: (u32, u32) = match args[1] {
          "id" => {
            unwrap_or_return!(
              self.find_piece_creation_id(unwrapres_or_return!(
                args[1].parse::<i32>(),
                RuleExpression::Err(format!("Not a valid id in {}", api))
              )),
              RuleExpression::Err("could not find piece with given id on board".to_string())
            )
          }
          "pos" | "position" => {
            unwrap_or_return!(
              self.get_valid_position(args[2]),
              RuleExpression::Err(String::from("not a valid position"))
            )
          }
          "var" | "variable" => {
            unwrap_or_return!(
              self.get_valid_position(unwrap_or_return!(
                context.get(args[2]),
                RuleExpression::Err(format!("no {} in {}", args[2], api))
              )),
              RuleExpression::Err(format!("{} is not a position {}", args[2], api))
            )
          }
          _ => {
            return RuleExpression::Err("wrong second argument".to_string());
          }
        };
        //let num = args[4].parse::<i32>().unwrap_or(0);
        if let Some(ref mut piece) = self.board[pos.0 as usize][pos.1 as usize] {
          match piece.memory.get(args[2]) {
            Some(val) => {
              let _ = match args[0] {
                "pair" => {
                  let parts: Vec<&str> = args[4].split(',').collect();
                  let pair = if parts.len() == 2 {
                    let x = parts[0].parse().unwrap_or(0);
                    let y = parts[1].parse().unwrap_or(0);
                    (x, y)
                  } else {
                    return RuleExpression::Err("fifth argument not a pair".to_string());
                  };
                  match val {
                    RuleExpression::Pair(a, b) => {
                      if let RuleExpression::Integer(a2) = **a {
                        if let RuleExpression::Integer(b2) = **b {
                          piece.memory.insert(
                            args[3].to_string(),
                            RuleExpression::Pair(
                              Box::new(RuleExpression::Integer(a2 + pair.0)),
                              Box::new(RuleExpression::Integer(b2 + pair.1)),
                            ),
                          );
                        } else {
                          RuleExpression::Err("given memory not a pair of integers".to_string());
                        }
                      } else {
                        RuleExpression::Err("given memory not a pair of integers".to_string());
                      }
                    }
                    _ => {}
                  }
                }
                "int" => {
                  let num = args[3].parse::<i32>();
                  if let RuleExpression::Integer(a) = val {
                    if num.is_ok() {
                      piece.memory.insert(
                        args[3].to_string(),
                        RuleExpression::Integer(num.unwrap() + a),
                      );
                    } else {
                      return RuleExpression::Err("fifth argument not an integer".to_string());
                    }
                  } else {
                    return RuleExpression::Err("given memory not an integer".to_string());
                  }
                }
                _ => {
                  return RuleExpression::Err("wrong first argument".to_string());
                }
              };
            }
            None => {
              return RuleExpression::Err("Given memory does not exist".to_string());
            }
          }
        }
        RuleExpression::Void
      }

      "increase_var" => {
        //increase_var [pair|int] [variable_name] value
        if args.len() != 3 {
          return RuleExpression::Err("wrong number of arguments".to_string());
        }
        let c = unwrap_or_return!(
          context.get(args[1]),
          RuleExpression::Err(format!("no {} in {}", args[1], api))
        );

        let _ = match args[0] {
          "pair" => {
            let parts: Vec<&str> = args[2].split(',').collect();
            let pair = if parts.len() == 2 {
              let x: Result<i32, _> = parts[0].parse();
              let y: Result<i32, _> = parts[1].parse();

              if x.is_err() || y.is_err() {
                return RuleExpression::Err("third argument not a pair of integers".to_string());
              }
              (x.unwrap(), y.unwrap())
            } else {
              return RuleExpression::Err("third argument not a pair".to_string());
            };

            let parts: Vec<&str> = c.split(',').collect();
            let pair2 = if parts.len() == 2 {
              let x: Result<i32, _> = parts[0].parse();
              let y: Result<i32, _> = parts[1].parse();

              if x.is_err() || y.is_err() {
                return RuleExpression::Err("given varable not a pair of integers".to_string());
              }
              (x.unwrap(), y.unwrap())
            } else {
              return RuleExpression::Err("given variable not a pair".to_string());
            };
            context.insert(
              args[1].to_string(),
              format!("{},{}", pair.0 + pair2.0, pair.1 + pair2.1),
            );
          }
          "int" => {
            let num = unwrapres_or_return!(
              args[2].parse::<i32>(),
              RuleExpression::Err("third argument not an integer".to_string())
            );
            let n2 = unwrapres_or_return!(
              c.parse::<i32>(),
              RuleExpression::Err("given variable not an integer".to_string())
            );
            context.insert(args[1].to_string(), format!("{}", n2 + num));
          }
          _ => {
            return RuleExpression::Err("wrong first argument".to_string());
          }
        };
        RuleExpression::Void
      }
      "change_type" => {
        // println!("{:?} {:?} {:?} {:?}", args, is_evaluate_mate, receiver, sender);
        if receiver.is_none() || sender.is_none() {
          // println!("bbb");
          return RuleExpression::Err(format!(
            "Cannot evaluate without receiver or sender in {}",
            api
          ));
        }
        if is_evaluate_mate {
          // println!("eeeeee");
          return RuleExpression::Err(format!("Cannot evaluate change_type in checking for mate"));
        }

        let matched: Vec<&str> = args
          .clone()
          .into_iter()
          .filter(|arg| self.pieces.contains_key(*arg))
          .collect();

        if matched.len() == 0 {
          return RuleExpression::Err(format!(
            "No pieces available to change to. Perhaps the listed pieces are not loaded in board."
          ));
        }
        // println!("promtp");
        self.send_response(
          vec![format!("Change to one of available types: {:?}", matched)].into_iter(),
          sender.unwrap(),
        );

        // println!("sent!");
        while let Ok(input) = receiver.unwrap().recv() {
          // println!("got it {:?}!!!",input);
          let input = input.trim();
          if matched.iter().any(|x| x.to_string() == input) {
            let mut new_piece = Piece::new(self.pieces[input].clone()).expect("a");
            let old_position = unwrap_or_return!(
              self.get_valid_position(unwrap_or_return!(
                context.get("new_position"),
                RuleExpression::Err(format!("no new_position in {}", api))
              )),
              RuleExpression::Err(format!("new_position in {}", api))
            );

            if let Some(old_piece) =
              self.board[old_position.0 as usize][old_position.1 as usize].take()
            {
              new_piece.memory = old_piece.memory;
              new_piece.owner = old_piece.owner;
              self.board[old_position.0 as usize][old_position.1 as usize] = Some(new_piece);
              return RuleExpression::Void;
            } else {
              return RuleExpression::Err(format!("No piece at {:?} in {}", old_position, api));
            }
          } else {
            self.send_response(
              vec![
                format!("Can't change to: {:?}", input),
                format!("Change to one of available types: {:?}", matched),
              ]
              .into_iter(),
              sender.unwrap(),
            );
          }
        }

        // println!("sth went wrong with inp");
        RuleExpression::Err("There were issues with input".to_string())
      }

      "forward" => {
        // forward [[position|variable] value ]| [history src|dest index]

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.get_valid_position(unwrap_or_return!(
            context.get("old_position"),
            RuleExpression::Err(format!("no old_position in {}", api))
          )),
          RuleExpression::Err(format!("old_position in {}", api))
        );

        let pos2 = unwrap_or_return!(
          self.match_and_get_pair(args, context),
          RuleExpression::Err(format!("could not get position in {}", api))
        ); //possible worse bc now not a specific error but more managable

        if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {
          // println!("{:?} {:?} {:?}",piece.owner, pos, pos2);
          match piece.owner {
            Player::White => return RuleExpression::Integer((pos.0 as i32) - (pos2.0 as i32)),
            Player::Black => return RuleExpression::Integer((pos2.0 as i32) - (pos.0 as i32)),
          }
        } else {
          return RuleExpression::Err(format!("No piece at {:?} position in {}", pos, api));
        }
      }
      "north" => {
        // north [[position|variable] value ]| [history src|dest index]

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.get_pair(unwrap_or_return!(
            context.get("old_position"),
            RuleExpression::Err(format!("no old_position in {}", api))
          )),
          RuleExpression::Err(format!("old_position in {}", api))
        );

        let pos2 = unwrap_or_return!(
          self.match_and_get_pair(args, context),
          RuleExpression::Err(format!("could not get position in {}", api))
        ); //possible worse bc now not a specific error but more managable

        return RuleExpression::Integer((pos.0 as i32) - (pos2.0 as i32));
      }
      "left" => {
        // left[[position|variable] value ]| [history src|dest index]

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.get_valid_position(unwrap_or_return!(
            context.get("old_position"),
            RuleExpression::Err(format!("no old_position in {}", api))
          )),
          RuleExpression::Err(format!("old_position in {}", api))
        );

        let pos2 = unwrap_or_return!(
          self.match_and_get_pair(args, context),
          RuleExpression::Err(format!("could not get position in {}", api))
        );

        if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {
          match piece.owner {
            Player::White => return RuleExpression::Integer((pos.1 as i32) - (pos2.1 as i32)),
            Player::Black => return RuleExpression::Integer((pos2.1 as i32) - (pos.1 as i32)),
          }
        } else {
          return RuleExpression::Err(format!("No piece at {:?} position in {}", pos, api));
        }
      }
      "west" => {
        // west [[position|variable] value ]|[history src|dest index]

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.get_pair(unwrap_or_return!(
            context.get("old_position"),
            RuleExpression::Err(format!("no old_position in {}", api))
          )),
          RuleExpression::Err(format!("old_position in {}", api))
        );

        let pos2 = unwrap_or_return!(
          self.match_and_get_pair(args, context),
          RuleExpression::Err(format!("could not get position in {}", api))
        );
        return RuleExpression::Integer((pos.1 as i32) - (pos2.1 as i32));
      }
      "history" => {
        // history src|dest inx (v|h)
        // TODO later(or when implementing a game that needs it) modify or add other commad for also taking a rule
        if args.len() < 2 || args.len() > 3 {
          return RuleExpression::Err(format!("wrong amount of argument in {}", api));
        }

        let pom = unwrap_or_return!(
          self.get_history(unwrapres_or_return!(
            args[1].parse::<u32>(),
            RuleExpression::Err(format!("wrong index in {}", api))
          )),
          RuleExpression::Err(format!("error with taking entry from history in {}", api))
        );

        let pom1 = match args[0] {
          "src" | "source" => pom.1 .0,
          "dst" | "dest" | "destination" => pom.1 .1,
          _ => {
            return RuleExpression::Err(format!("wrong first argument in {}", api));
          }
        };

        if args.len() == 2 {
          return RuleExpression::Pair(
            Box::new(RuleExpression::Integer(pom1.0 as i32)),
            Box::new(RuleExpression::Integer(pom1.1 as i32)),
          );
        }

        match args[2] {
          "v" => RuleExpression::Integer(pom1.0 as i32),
          "h" => RuleExpression::Integer(pom1.1 as i32),
          _ => RuleExpression::Err(format!("wrong third argument in {}", api)),
        }
      }

      //kills piece at specified position
      "kill" => {
        // kill [[position|variable] value ]| [history src|dest index]
        if receiver.is_none() || sender.is_none() {
          return RuleExpression::Err(format!(
            "Cannot evaluate without receiver or sender in {}",
            api
          ));
        }

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.match_and_get_valid_position(args, context),
          RuleExpression::Err(format!("could not get position in {}", api))
        );

        if let Some(ref piece3) = self.board[pos.0 as usize][pos.1 as usize] {
          let mut piece2 = piece3.clone();
          piece2.owner = !piece3.owner;
          match piece2.owner {
            Player::White => self.cementaries.0.push(piece2.clone()), // White's cemetery
            Player::Black => self.cementaries.1.push(piece2.clone()), // Black's cemetery
          }

          evaluate_rules!(
            self,
            (pos.0, pos.1),
            deathrattle,
            context,
            depth,
            sender,
            receiver,
            is_evaluate_mate
          );
        }

        self.board[pos.0 as usize][pos.1 as usize] = None;
        RuleExpression::Void
      }

      //TODO decide whether to add remove and destroy
      //remove piece at specified position without deathrattle
      "remove" => {
        // kill [[position|variable] value ]| [history src|dest index]
        if receiver.is_none() || sender.is_none() {
          return RuleExpression::Err(format!(
            "Cannot evaluate without receiver or sender in {}",
            api
          ));
        }

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.match_and_get_valid_position(args, context),
          RuleExpression::Err(format!("could not get position in {}", api))
        );

        if let Some(ref piece3) = self.board[pos.0 as usize][pos.1 as usize] {
          let mut piece2 = piece3.clone();
          piece2.owner = !piece3.owner;
          match piece2.owner {
            Player::White => self.cementaries.0.push(piece2.clone()), // White's cemetery
            Player::Black => self.cementaries.1.push(piece2.clone()), // Black's cemetery
          }
        }

        self.board[pos.0 as usize][pos.1 as usize] = None;
        RuleExpression::Void
      }

      "destroy" => {
        // kill [[position|variable] value ]| [history src|dest index]
        if receiver.is_none() || sender.is_none() {
          return RuleExpression::Err(format!(
            "Cannot evaluate without receiver or sender in {}",
            api
          ));
        }

        if !((args.len() == 2 && (args[0] != "history" && args[0] != "h"))
          || (args.len() == 3 && (args[0] == "history" || args[0] == "h")))
        {
          return RuleExpression::Err(format!(
            "wrong number of arguments in {} args:{}",
            api,
            args.len()
          ));
        }

        let pos = unwrap_or_return!(
          self.match_and_get_valid_position(args, context),
          RuleExpression::Err(format!("could not get position in {}", api))
        );

        self.board[pos.0 as usize][pos.1 as usize] = None;
        RuleExpression::Void
      }

      // args: [piece name ] piece corresponds to id field in piece(zmień płeć)
      "piece_on_board" => {
        if !self.pieces.contains_key(args[0]) {
          return RuleExpression::Err(format!(
            "Piece {} does not exist in this game, in {}",
            args[0], api
          ));
        }

        RuleExpression::Boolean(self.board.iter().any(|x| {
          x.iter()
            .any(|y| y.as_ref().is_some_and(|z| z.id == args[0]))
        }))
      }

      "piece_on_board_cnt" => {
        if !self.pieces.contains_key(args[0]) {
          return RuleExpression::Err(format!(
            "Piece {} does not exist in this game, in {}",
            args[0], api
          ));
        }

        RuleExpression::Integer(
          self
            .board
            .iter()
            .flat_map(|row| row.iter())
            .filter(|&piece| piece.as_ref().is_some_and(|v| v.id == args[0]))
            .count() as i32,
        )
      }
      // args: [piece name] [player]
      "player_piece_on_board" => {
        if !self.pieces.contains_key(args[0]) {
          return RuleExpression::Err(format!(
            "Piece {} does not exist in this game, in {}",
            args[0], api
          ));
        }

        let pp;
        if args.len() > 1 {
          pp = unwrap_or_return!(
            Player::from_str(args[1]),
            RuleExpression::Err("Please add a valid player".to_string())
          );
        } else {
          pp = self.current_player
        }

        RuleExpression::Boolean(self.board.iter().any(|x| {
          x.iter()
            .any(|y| y.as_ref().is_some_and(|z| z.id == args[0] && z.owner == pp))
        }))
      }

      "player_piece_on_board_cnt" => {
        if !self.pieces.contains_key(args[0]) {
          return RuleExpression::Err(format!(
            "Piece {} does not exist in this game, in {}",
            args[0], api
          ));
        }
        let pp;
        if args.len() > 1 {
          pp = unwrap_or_return!(
            Player::from_str(args[1]),
            RuleExpression::Err("Please add a valid player".to_string())
          );
        } else {
          pp = self.current_player
        }
        RuleExpression::Integer(
          self
            .board
            .iter()
            .flat_map(|row| row.iter())
            .filter(|&piece| {
              piece
                .as_ref()
                .is_some_and(|v| v.id == args[0] && v.owner == pp)
            })
            .count() as i32,
        )
      }

      //args: [pos position1 position2] [var var_position1 var_position2] position1 must have a piece and position2 must be empty
      "move_piece" => {
        if args.len() != 3 {
          return RuleExpression::Err("wrong number of arguments".to_string());
        }

        let pos1;
        let pos2;
        if args[0] == "var" {
          let var1 = unwrap_or_return!(
            context.get(args[1]),
            RuleExpression::Err("context does not contain var1".to_string())
          );
          let var2 = unwrap_or_return!(
            context.get(args[2]),
            RuleExpression::Err("context does not contain var2".to_string())
          );

          pos1 = unwrap_or_return!(
            self.get_valid_position(var1),
            RuleExpression::Err("position 1 is not valid".to_string())
          );
          pos2 = unwrap_or_return!(
            self.get_valid_position(var2),
            RuleExpression::Err("position 2 is not valid".to_string())
          );
        } else if args[0] == "pos" {
          pos1 = unwrap_or_return!(
            self.get_valid_position(args[1]),
            RuleExpression::Err("position 1 is not valid".to_string())
          );
          pos2 = unwrap_or_return!(
            self.get_valid_position(args[2]),
            RuleExpression::Err("position 2 is not valid".to_string())
          );
        } else {
          return RuleExpression::Err(format!("Wrong argument to move_piece: {}", args[0]));
        }

        if self.board[pos2.0 as usize][pos2.1 as usize].is_some() {
          return RuleExpression::Err("moving a piece to occupied position".to_string());
        }

        let piece = unwrap_or_return!(
          self.board[pos1.0 as usize][pos1.1 as usize].clone(),
          RuleExpression::Err("No piece at given position".to_string())
        );

        self.board[pos2.0 as usize][pos2.1 as usize] = Some(piece);
        self.board[pos1.0 as usize][pos1.1 as usize] = None;
        RuleExpression::Void
      }

      //args: position1 position2 [piece|owner]
      "is_same_line" => {
        if args.len() != 3 {
          return RuleExpression::Err("wrong number of arguments".to_string());
        }

        let pos1 = unwrap_or_return!(
          self.get_valid_position(args[0]),
          RuleExpression::Err("position 1 is not valid".to_string())
        );
        let pos2 = unwrap_or_return!(
          self.get_valid_position(args[1]),
          RuleExpression::Err("position 1 is not valid".to_string())
        );

        let x = (pos2.0 as i32) - (pos1.0 as i32);
        let y = (pos2.1 as i32) - (pos1.1 as i32);

        if x != 0 && y != 0 && x.abs() != y.abs() {
          return RuleExpression::Err(
            "Positions not on straight line or a square diagonal".to_string(),
          );
        }

        if x == 0 && y == 0 {
          return RuleExpression::Boolean(true);
        }
        let diff = (
          x / std::cmp::max(x.abs(), y.abs()),
          y / std::cmp::max(x.abs(), y.abs()),
        );
        let mut i = 0;
        let mut pos3 = pos1;
        //println!("polj: {:?}, {:?}, p3 {:?}, p1 {:?}, p2 {:?}, {}",diff,i,pos3,pos1, pos2, args[2]);
        while pos3 != pos2 {
          // println!("a: {:?}, {:?}, p3 {:?}, p1 {:?}, p2 {:?}, {}",diff,i,pos3,pos1, pos2, args[2]);
          if self.board[pos3.0 as usize][pos3.1 as usize].is_none()
            && self.board[pos2.0 as usize][pos2.1 as usize].is_none()
          {
            return RuleExpression::Boolean(false);
          }
          if self.board[pos3.0 as usize][pos3.1 as usize].is_some()
            && self.board[pos2.0 as usize][pos2.1 as usize].is_some()
          {
            if match args[2] {
              "piece" => {
                self.board[pos3.0 as usize][pos3.1 as usize]
                  .as_ref()
                  .unwrap()
                  .id
                  != self.board[pos2.0 as usize][pos2.1 as usize]
                    .as_ref()
                    .unwrap()
                    .id
              }
              "owner" => {
                self.board[pos3.0 as usize][pos3.1 as usize]
                  .as_ref()
                  .unwrap()
                  .owner
                  != self.board[pos2.0 as usize][pos2.1 as usize]
                    .as_ref()
                    .unwrap()
                    .owner
              }
              _ => {
                return RuleExpression::Err("unknown argument".to_string());
              }
            } {
              return RuleExpression::Boolean(false);
            }
          } else {
            return RuleExpression::Boolean(false);
          }

          i += 1;
          pos3 = (
            ((pos1.0 as i32) + i * diff.0) as u32,
            ((pos1.1 as i32) + i * diff.1) as u32,
          );
          //println!("e {:?}, p3 {:?}",i, pos3);
        }
        RuleExpression::Boolean(true)
      }
      "check_mate" => {
        if args.len() != 2 {
          return RuleExpression::Err("wrong number of arguments".to_string());
        }
        let piece_id = unwrapres_or_return!(
          args[0].parse::<i32>(),
          RuleExpression::Err("Not a valid piece id".to_string())
        );
        let player = unwrap_or_return!(
          Player::from_str(args[1]),
          RuleExpression::Err("Not a valid player".to_string())
        );
        return self.check_mate(player, piece_id);
      }
      "create_var_from_mem" => {
        //create_var_from_mem [[pos position]|[id piece_id ]| [var position_name]] [memory_name] [variable_name]
        // println!("b");
        if args.len() != 4 {
          return RuleExpression::Err("wrong number of arguments".to_string());
        }
        let position: (u32, u32) = match args[0] {
          "pos" => {
            unwrap_or_return!(
              self.get_valid_position(args[1]),
              RuleExpression::Err("position 1 is not valid".to_string())
            )
          }
          "id" => {
            let piece_id = unwrapres_or_return!(
              args[1].parse::<i32>(),
              RuleExpression::Err("Not a valid piece id".to_string())
            );
            let (x, y) = match self.find_piece_creation_id(piece_id) {
              Some(a) => a,
              None => {
                return RuleExpression::Boolean(true);
              }
            };
            (x, y)
          }
          "var" => {
            // println!("cttt {:?}",context.get(args[1]));
            unwrap_or_return!(
              self.get_valid_position(unwrap_or_return!(
                context.get(args[1]),
                RuleExpression::Err(format!("no given position in {}", api))
              )),
              RuleExpression::Err(format!("position in {}", api))
            )
          }
          _ => {
            return RuleExpression::Err("wrong first argument".to_string());
          }
        };
        let memory = self.get_piece_var(
          position,
          args[2],
          context,
          depth,
          sender,
          receiver,
          is_evaluate_mate,
        );
        match memory {
          RuleExpression::Err(e) => {
            return RuleExpression::Err(e);
          }
          _ => {
            //let m2 = memory.evaluate(self, context,depth+1 ,sender ,receiver ,is_evaluate_mate);
            context.insert(args[3].to_string(), memory.string2());
          }
        }
        RuleExpression::Void
      }

      "create_var_from_history" => {
        //create_var_from_history [src|dst] [vertical|horizontal|pair] [idx] [variable_name]
        if args.len() != 4 {
          return RuleExpression::Err("wrong number of arguments".to_string());
        }
        let pomi = unwrap_or_return!(
          self.get_history(unwrapres_or_return!(
            args[2].parse::<u32>(),
            RuleExpression::Err(format!("wrong index in {}", api))
          )),
          RuleExpression::Err(format!("error with taking entry from history in {}", api))
        );

        let p2 = match args[0] {
          "src" | "source" => pomi.1 .0,
          "dst" | "dest" | "destination" => pomi.1 .1,
          _ => {
            return RuleExpression::Err("Wrong first argument".to_string());
          }
        };

        match args[1] {
          "vertical" | "v" | "vert" => {
            context.insert(args[3].to_string(), format!("{}", p2.0));
          }
          "horizontal" | "h" | "horz" => {
            context.insert(args[3].to_string(), format!("{}", p2.1));
          }
          "pair" | "p" => {
            context.insert(args[3].to_string(), format!("{},{}", p2.0, p2.1));
          }
          _ => {
            return RuleExpression::Err("Wrong second argument".to_string());
          }
        };

        RuleExpression::Void
      }
      "clone" => {
        // clone [variable_name1] [variable_name2]
        if args.len() != 2 {
          return RuleExpression::Err(format!("wrong number of argumenst"));
        }
        let c = unwrap_or_return!(
          context.get(args[0]),
          RuleExpression::Err(format!("no {} in {}", args[0], api))
        );

        context.insert(args[1].to_string(), c.clone());
        RuleExpression::Void
      }
      not_found => RuleExpression::Err(format!("unknown call: {}", not_found)),
    }
  }

  fn find_piece_creation_id(&self, piece_id: i32) -> Option<(u32, u32)> {
    for (i, row) in (&self.board).iter().enumerate() {
      for (j, square) in row.iter().enumerate() {
        if let Some(piece) = square {
          if piece.memory.get("id").unwrap() == &RuleExpression::Integer(piece_id) {
            return Some((i as u32, j as u32));
          }
        }
      }
    }
    None
  }

  pub fn check_mate(&mut self, player: Player, piece_id: i32) -> RuleExpression {
    let start = Instant::now();
    let (x, y) = match self.find_piece_creation_id(piece_id) {
      Some(a) => a,
      None => {
        //println!("if piece doesn't exist, it's probably been mated already?");
        return RuleExpression::Boolean(true);
      }
    };

    let mut board_clone = self.clone();
    // let oldown = board_clone.current_player;
    board_clone.current_player = !player;
    if !board_clone.is_attacked((x, y), 0) {
      //println!("not attacked, {},{}, curr={:?}",x,y,board_clone.current_player);
      return RuleExpression::Boolean(false);
    }
    board_clone.current_player = player;
    //println!("piece is attacked, {},{}", x, y);

    //timeout 2000ms
    if start.elapsed().as_millis() > 2000 {
      return RuleExpression::Err(String::from("Timeout"));
    }

    for (i, row) in (&self.board).iter().enumerate() {
      for (j, square) in row.iter().enumerate() {
        if let Some(piece) = square {
          if piece.owner == player {
            // println!("{:?} exists at {:?} and belongs to {:?}", piece.name, (i,j), player);

            //get all moves for that piece, if they exist
            let moves = board_clone.get_possible_moves((i as u32, j as u32), 0);
            if let Some(move1) = moves {
              for move2 in move1 {
                // println!("move: {:?}", move2);
                //for every move, clone the board and make that move as if it's our turn
                let mut new_board = board_clone.clone();
                new_board.current_player = player;
                let (sender1, _receiver1) = std::sync::mpsc::channel();
                let (_sender2, receiver2) = std::sync::mpsc::channel();
                match new_board.make_move((i as u32, j as u32), move2.0, move2.1, &sender1, &receiver2, false, true) // tu może coś z głębokością????
                                {
                                    Ok(c) => {
                                        // println!("ok: {:?}", c);
                                        match c {
                                            GameState::BlackWins => {
                                                if player==Player::Black{
                                                    return RuleExpression::Boolean(true);
                                                }
                                            }
                                            GameState::WhiteWins =>{
                                                if player==Player::White{
                                                    return RuleExpression::Boolean(true);
                                                }
                                            }
                                            GameState::Draw => {
                                                return RuleExpression::Boolean(true);
                                            }
                                            _ => ()
                                        }
                                    }
                                    Err(_e) => {
                                        // println!("failed to make the move because {:?}, so it doesn't get us out of mate", _e);
                                        continue;
                                    }
                                };
                let (x, y) = match new_board.find_piece_creation_id(piece_id) {
                  Some(a) => a,
                  None => {
                    // println!("target piece disappeared, so this is not a move that gets us out of mate");
                    continue;
                  }
                };
                new_board.current_player = !player;
                if !new_board.is_attacked((x, y), 0) {
                  // println!("There exists a move that gets us out of mate");
                  // println!("new board:\n{new_board}");
                  return RuleExpression::Boolean(false);
                }
                // println!("target piece is still attacked after move");

                if start.elapsed().as_millis() > 2000 {
                  return RuleExpression::Err(String::from("Timeout"));
                }
              }
            }
            // println!("out")
          }
        }
      }
    }
    // println!("tested moves!");

    //test revives
    let cement = if player == Player::White {
      self.cementaries.1.clone()
    } else {
      self.cementaries.0.clone()
    };
    for (idx, _piece) in cement.iter().enumerate() {
      for row in 0..self.size.0 {
        for col in 0..self.size.1 {
          let mut new_board = board_clone.clone();
          new_board.current_player = player;
          let (sender1, _receiver1) = std::sync::mpsc::channel();
          let (_sender2, receiver2) = std::sync::mpsc::channel();

          match new_board.revive_piece(
            player == Player::White,
            idx as u32,
            (row as u32, col as u32),
            &sender1,
            &receiver2,
            true,
          ) {
            Ok(c) => match c {
              GameState::BlackWins => {
                if player == Player::Black {
                  return RuleExpression::Boolean(true);
                }
              }
              GameState::WhiteWins => {
                if player == Player::White {
                  return RuleExpression::Boolean(true);
                }
              }
              GameState::Draw => {
                return RuleExpression::Boolean(true);
              }
              _ => (),
            },
            Err(_e) => {
              // println!("failed to make the revive because {:?}, so it doesn't get us out of mate", _e);
              continue;
            }
          };
          let (x, y) = match new_board.find_piece_creation_id(piece_id) {
            Some(a) => a,
            None => {
              // println!("target piece disappeared, so this is not a move that gets us out of mate");
              continue;
            }
          };
          new_board.current_player = !player;
          if !new_board.is_attacked((x, y), 0) {
            // println!("There exists a move that gets us out of mate");
            // println!("new board:\n{new_board}");
            return RuleExpression::Boolean(false);
          }
          // println!("target piece is still attacked after revive");

          if start.elapsed().as_millis() > 2000 {
            return RuleExpression::Err(String::from("Timeout"));
          }
        }
      }
    }
    // println!("we ran out of options, all moves end in a mate");
    RuleExpression::Boolean(true)
  }

  ///zamienia pozycję z &str na (u32,u32)
  pub fn get_pair(&self, pos: &str) -> Option<(i32, i32)> {
    self.parse_pair(&pos)
  }

  pub fn get_valid_position(&self, pos: &str) -> Option<(u32, u32)> {
    let a = self.parse_position(&pos);
    if a.is_some() {
      if a.unwrap().0 < self.size.0 || a.unwrap().1 < self.size.1 {
        return a;
      }
    }
    None
  }

  ///zmienia pozyscje pos1 z &str na (u32,u32). przy tym jest względna
  pub fn get_position_from_relative(&self, pos1: &str, pos2: (u32, u32)) -> Option<(u32, u32)> {
    let p = unwrap_or_return!(self.parse_position(pos1), None);
    let p2 = ((p.0 + pos2.0) as u32, (p.1 + pos2.1) as u32);
    if p2.0 < self.size.0 && p2.1 < self.size.1 {
      return Some(p2);
    }
    None
  }

  ///zmienia pozyscje pos1 z &str na (u32,u32). przy tym jest względna
  pub fn get_position_from_pawn_relative(
    &self,
    pos1: &str,
    pos2: (u32, u32),
    white: bool,
  ) -> Option<(u32, u32)> {
    let p = unwrap_or_return!(self.parse_position(pos1), None);
    if white {
      let p2 = ((p.0 + pos2.0) as u32, (p.1 + pos2.1) as u32);
      if p2.0 < self.size.0 && p2.1 < self.size.1 {
        return Some(p2);
      }
      return None;
    }
    if p.0 < pos2.0 || p.1 < pos2.1 {
      return None;
    }
    Some(((p.0 - pos2.0) as u32, (p.1 - pos2.1) as u32))
  }

  pub fn get_game_state(
    &mut self,
    context: &mut HashMap<String, String>,
  ) -> Result<GameState, String> {
    let mut ec = false;
    if self.endcondition.is_some() {
      let end = self
        .endcondition
        .as_ref()
        .unwrap()
        .clone()
        .evaluate(self, context, 0, None, None, false);
      ec = true;
      match end {
        RuleExpression::Boolean(x) => {
          if !x {
            //  println!("no end");
            self.current_player = !self.current_player;
            return Ok(GameState::Continue);
          }
        }
        _ => {
          return Err("In make move: sth went wrong, end result not bool".to_string());
        }
      }
    }

    let white: RuleExpression = self
      .wincondition
      .0
      .clone()
      .evaluate(self, context, 0, None, None, false);
    let black: RuleExpression = self
      .wincondition
      .1
      .clone()
      .evaluate(self, context, 0, None, None, false);
    match (white.clone(), black.clone()) {
      (RuleExpression::Boolean(w), RuleExpression::Boolean(b)) => {
        if w == b {
          if ec || w {
            return Ok(GameState::Draw);
          }

          self.current_player = !self.current_player;
          return Ok(GameState::Continue);
        }
        if w {
          return Ok(GameState::WhiteWins);
        }
        return Ok(GameState::BlackWins);
      }
      _ => {
        println!("winconds: {:?} {:?}", white, black);
        return Err("sth went wrong, one of the winconditions not bool".to_string());
      }
    }
  }

  pub fn make_move(
    &mut self,
    old_position: (u32, u32),
    new_position: (u32, u32),
    pmove: PMove,
    sender: &Sender<Option<String>>,
    receiver: &Receiver<String>,
    evaluate_endconditions: bool,
    is_evaluate_mate: bool,
  ) -> Result<GameState, String> {
    let (x1, y1) = old_position;
    let (x2, y2) = new_position;
    let mut board_clone = self.clone();
    if x1 >= self.size.0 || y1 >= self.size.1 || x2 >= self.size.0 || y2 >= self.size.1 {
      return Err("Out of board".to_string());
    }
    if self.board[x1 as usize][y1 as usize].is_none() {
      return Err("Piece doesn't exist".to_string());
    }

    let mut context = HashMap::new();
    context.insert(
      String::from("new_position"),
      format!("{},{}", x2, y2).to_string(),
    );
    context.insert(
      String::from("old_position"),
      format!("{},{}", x1, y1).to_string(),
    );

    let movecondition = self.board[x1 as usize][y1 as usize]
      .clone()
      .unwrap()
      .movecondition
      .clone();
    let condition_met = movecondition.as_ref().map_or(true, |condition| {
      matches!(
        condition.evaluate(
          &mut board_clone,
          &mut context,
          0,
          Some(sender),
          Some(receiver),
          is_evaluate_mate
        ),
        RuleExpression::Boolean(true)
      )
    });
    board_clone = self.clone();
    let move_possible = matches!(
      pmove.condition.evaluate(
        &mut board_clone,
        &mut context,
        0,
        Some(sender),
        Some(receiver),
        is_evaluate_mate
      ),
      RuleExpression::Boolean(true)
    );

    if !condition_met || !move_possible {
      return Err("Cannot perform move".to_string());
    }

    evaluate_rules!(
      self,
      (x1, y1),
      onmove,
      &mut context,
      0,
      Some(sender),
      Some(receiver),
      is_evaluate_mate
    );
    let mut kill = false;
    if let Some(ref piece3) = self.board[x2 as usize][y2 as usize] {
      //TODO add if piece can be killed, for example if i can kill my own field
      let mut piece2 = piece3.clone();
      piece2.owner = !piece3.owner;
      match piece2.owner {
        Player::White => self.cementaries.0.push(piece2.clone()), // White's cemetery
        Player::Black => self.cementaries.1.push(piece2.clone()), // Black's cemetery
      }
      kill = true;
    } else {
      // leave it for now since we might want to do actions only if not kill
    }
    if kill {
      evaluate_rules!(
        self,
        (x2, y2),
        deathrattle,
        &mut context,
        0,
        Some(sender),
        Some(receiver),
        is_evaluate_mate
      );
    }

    let temp = std::mem::take(&mut self.board[x1 as usize][y1 as usize]);
    self.board[x2 as usize][y2 as usize] = temp;
    self.board[x1 as usize][y1 as usize] = None;

    if kill {
      evaluate_rules!(
        self,
        (x2, y2),
        onkill,
        &mut context,
        0,
        Some(sender),
        Some(receiver),
        is_evaluate_mate
      );
    }

    if let Some(ref cons) = pmove.consequences {
      cons.iter().for_each(|x| {
        let _ = x.evaluate(
          self,
          &mut context,
          0,
          Some(sender),
          Some(receiver),
          is_evaluate_mate,
        );
      });
    }

    for x6 in 0..self.size.0 {
      for y6 in 0..self.size.1 {
        evaluate_rules!(
          self,
          (x6, y6),
          passive,
          &mut context,
          0,
          Some(sender),
          Some(receiver),
          is_evaluate_mate
        );
      }
    }

    evaluate_rules!(
      self,
      (x2, y2),
      aftermove,
      &mut context,
      0,
      Some(sender),
      Some(receiver),
      is_evaluate_mate
    );

    self.add_history(pmove.clone(), old_position, new_position);

    if evaluate_endconditions {
      self.get_game_state(&mut context)
    } else {
      Ok(GameState::Continue)
    }
  }

  fn send_response<I>(&self, strings: I, sender: &Sender<Option<String>>)
  where
    I: Iterator<Item = String>,
  {
    for str in strings {
      sender.send(Some(str)).expect("Failed to send response");
    }
    sender.send(None).expect("Failed to send end of response");
  }
}

impl std::fmt::Display for Board {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let string = self
      .board
      .iter()
      .map(|row| {
        row
          .iter()
          .map(|cell| match cell {
            Some(piece) => piece.to_string(),
            None => ".".to_string(),
          })
          .collect::<Vec<String>>()
          .join(" ")
      })
      .collect::<Vec<String>>()
      .join("\n");
    write!(f, "{}", string)
  }
}
