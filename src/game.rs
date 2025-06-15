use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::board::{default_calls, Board};
use crate::player::Player;
use crate::GameState;
use crate::{PMove, Piece, RuleExpression};

macro_rules! send_response_and_return {
  ($self:ident, $msg:expr, $stay:expr) => {{
    $self.send_response(vec![$msg.to_string()].into_iter());
    return $stay;
  }};
}

#[derive(Clone, Debug)]
pub enum RuleState {
  Closed(RuleExpression),
  Open {
    name: String,
    children: Vec<RuleState>,
    size: i32,
  },
  Empty,
}

fn print_rule_state(expr2: RuleState) -> String {
  let mut result;
  // println!("db print: {:?}",expr2);
  match expr2 {
    RuleState::Closed(rexpr) => {
      format!("{:?}", rexpr)
    }
    RuleState::Open {
      name,
      children,
      size,
    } => {
      result = format!("{}(", name);
      for c in children.clone() {
        result.push_str(&format!("{},", print_rule_state(c)));
      }
      if size == -1 {
        result.push_str("additional RuleExpressions can be added)");
      } else {
        //if it is open then either there is not a limit to the number of expressions or children.len<size
        let left = size - (children.len() as i32);
        result.push_str("empty,".repeat(left as usize).as_str());
        result.pop();
        result.push(')');
      }
      result
    }
    RuleState::Empty => "Empty rule".to_string(),
  }
}

fn add_node(node: &mut RuleState, path: Vec<usize>, depth: usize, child: RuleState) {
  if depth >= path.len() {
    match node {
      RuleState::Empty => {
        *node = child;
      }
      RuleState::Open { children, .. } => {
        children.push(child);
      }
      _ => {}
    }
  } else {
    if let RuleState::Open { children, .. } = node {
      let ind = path[depth];
      if ind < children.len() {
        return add_node(&mut children[ind], path, depth + 1, child);
      } else if ind == children.len() {
        children.push(child);
      }
    }
  }
}

fn pop_node(node: &mut RuleState, path: Vec<usize>, depth: usize) {
  if depth + 1 >= path.len() {
    match node {
      RuleState::Open { children, .. } => {
        children.pop();
      }
      RuleState::Closed(..) => {
        *node = RuleState::Empty;
      }
      _ => {}
    }
  } else {
    if let RuleState::Open { children, .. } = node {
      let ind = path[depth];
      if ind < children.len() {
        return pop_node(&mut children[ind], path, depth + 1);
      }
    }
  }
}

fn swap_node(node: &mut RuleState, path: Vec<usize>, depth: usize, closed: RuleState) {
  // println!("db: node: {:?} path: {:?} depth:{:?}, closed: {:?}", node, path, depth, closed);
  if depth >= path.len() {
    *node = closed;
  } else {
    if let RuleState::Open { children, .. } = node {
      let ind = path[depth];
      if ind < children.len() {
        return swap_node(&mut children[ind], path, depth + 1, closed);
      }
    }
  }
}
///reprezentuje obecny stan gry
#[derive(Debug, PartialEq)]
pub enum Mode {
  Start,
  InitBoard,
  InitPieces,
  PlacePieces,
  Playing,
  Finished,
  CreateBoard,
}

impl Mode {
  pub fn to_string(&self) -> String {
    match self {
      Mode::Start => "start".to_string(),
      Mode::InitBoard => "initboard".to_string(),
      Mode::InitPieces => "initpieces".to_string(),
      Mode::PlacePieces => "placepieces".to_string(),
      Mode::Playing => "playing".to_string(),
      Mode::Finished => "finished".to_string(),
      Mode::CreateBoard => "createboard".to_string(),
    }
  }
}
///reprezentuje grę
pub struct Game {
  board: Option<Board>,

  sender: Sender<Option<String>>,
  receiver: Receiver<String>,

  current_mode: Mode,
  tui: bool,
  json: Receiver<String>,
}

impl Game {
  ///uruchamia wątek gry
  pub fn start_game(
    rx: Receiver<String>,
    tx: Sender<Option<String>>,
    tui: bool,
  ) -> (thread::JoinHandle<()>, Sender<String>) {
    let (tx_json, rx_json) = mpsc::channel();
    (
      thread::spawn(move || {
        let mut game = Game::new(rx, tx, rx_json, tui);

        game.game_loop();
      }),
      tx_json,
    )
  }

  ///główna pętla programu
  fn game_loop(&mut self) {
    while let Ok(msg) = self.receiver.recv() {
      println!("[Game] Received input: {}", msg);

      if msg == String::from("show") {
        self.show_board();
        continue;
      }

      if msg == String::from("end") {
        break;
      }

      if msg == String::from("getstatus") {
        self
          .sender
          .send(Some(self.current_mode.to_string()))
          .expect("Failed to send status");
        continue;
      }

      if msg == String::from("getdimensions") {
        self.get_dimensions();
        continue;
      }

      if msg == String::from("currentwhite") {
        self.currentwhite();
        continue;
      }

      if msg.starts_with("verifypiece") {
        self.verifypiece(msg);
        continue;
      }

      self.current_mode = match self.current_mode {
        Mode::Start => self.handle_start(msg),
        Mode::InitBoard => self.handle_init_board(msg),
        Mode::InitPieces => self.handle_init_pieces(msg),
        Mode::PlacePieces => self.handle_place_pieces(msg),
        Mode::Playing => self.handle_playing(msg),
        Mode::Finished => self.handle_finished(msg),
        Mode::CreateBoard => self.handle_create_board(msg),
      };
    }
  }

  ///tworzy instancję gry
  pub fn new(
    rx: Receiver<String>,
    tx: Sender<Option<String>>,
    rx_json: Receiver<String>,
    tui: bool,
  ) -> Self {
    Self {
      board: None,
      receiver: rx,
      sender: tx,
      current_mode: Mode::Start,
      json: rx_json,
      tui,
    }
  }

  fn send_response<I>(&self, strings: I)
  where
    I: Iterator<Item = String>,
  {
    for str in strings {
      self
        .sender
        .send(Some(str))
        .expect("Failed to send response");
    }
    self
      .sender
      .send(None)
      .expect("Failed to send end of response");
  }

  fn extractnum(&self, num: &RuleExpression) -> String {
    //TODO change the error handling here currently possibly to overwrite id and break sth
    match num {
      RuleExpression::Integer(n) => n.to_string(),
      _ => "_".to_string(),
    }
  }

  fn show_cementary(&self, cementary: &Vec<Piece>) -> String {
    let cmt: Vec<String> = cementary
      .iter()
      .map(|x| {
        format!(
          "({},{})",
          x.name,
          self.extractnum(x.memory.get("id").unwrap())
        )
      })
      .collect();
    cmt.join(",")
  }

  fn show_board(&self) {
    match &self.board {
      Some(board) => {
        let board_str = format!("{}", board);

        let cementaries_strs = (
          format!(
            "white: {}",
            self.show_cementary(board.cementaries.0.as_ref())
          ),
          format!(
            "black: {}",
            self.show_cementary(board.cementaries.1.as_ref())
          ),
        );
        let mut board_lines = board_str
          .lines()
          .map(|line| line.to_string())
          .collect::<Vec<String>>();
        if self.tui {
          board_lines.push(cementaries_strs.0);
          board_lines.push(cementaries_strs.1);
        }
        self.send_response(board_lines.into_iter());
      }
      None => {
        self.send_response(vec!["No board initialized yet.".to_string()].into_iter());
      }
    }
  }

  fn get_dimensions(&self) {
    match &self.board {
      Some(board) => {
        let size = format!("{},{}", board.size.0, board.size.1).to_string();
        let v = vec![size];
        self.send_response(v.into_iter());
      }
      None => {
        self.send_response(vec!["No board initialized yet.".to_string()].into_iter());
      }
    }
  }

  fn verifypiece(&mut self, msg: String) {
    let json = msg.trim_start_matches("verifypiece ").trim();
    let args: Vec<&str> = json.split_whitespace().collect();
    if args.len() != 1 {
      self.send_response(vec!["Wrong input.\n verifypiece [json]".to_string()].into_iter());
    }

    match self.load_pieces_json(json.to_string()) {
      Ok(_) => self.send_response(vec!["Piece loaded from json".to_string()].into_iter()),
      Err(e) => self.send_response(
        vec![
          format!("Failed to load piece: {}", e),
          "Try again or load another piece.".to_string(),
        ]
        .into_iter(),
      ),
    }
  }

  fn currentwhite(&self) {
    let ret = if self.board.as_ref().unwrap().current_player == Player::White {
      "White"
    } else {
      "Black"
    };
    self.send_response(vec![ret.to_string()].into_iter());
  }

  fn handle_start(&self, _msg: String) -> Mode {
    self.send_response(
      vec![
        "Welcome to the Board Games!".to_string(),
        "Choose how to initialize the board:".to_string(),
        "- 'default': Create a default board".to_string(),
        "- 'load <filename>': Load a board from a file".to_string(),
        "- 'load_json': Load a board from json".to_string(),
        "- 'create': enter boardcreator".to_string(),
        "- 'help': list available commands".to_string(),
      ]
      .into_iter(),
    );
    Mode::InitBoard
  }

  /// Handle board initialization state
  fn handle_init_board(&mut self, msg: String) -> Mode {
    let stay = Mode::InitBoard;
    match msg.as_str() {
      "default" => {
        self.new_default_board();
        self.send_response(
          vec![
            "Default board created successfully!".to_string(),
            "Next: Initialize pieces".to_string(),
            "- 'continue': Go to piece placement".to_string(),
            "- 'load <filename>: Load a piece from a file".to_string(),
            "- 'load_json': Load a piece from json".to_string(),
            "- 'create': enter piece creator".to_string(),
            "- 'help': list available commands".to_string(),
            "Tip: Use 'show' at any point to view the board".to_string(),
          ]
          .into_iter(),
        );
        Mode::InitPieces
      }
      "create" => {
        self.new_default_board();
        self.send_response(
          vec![
            "Default board created successfully!".to_string(),
            "Next: Edit board".to_string(),
            "- 'continue': Go to piece placement".to_string(),
            "- show size".to_string(),
            "- show history_size".to_string(),
            "- show end_condition".to_string(),
            "- show win_condition".to_string(),
            "- show revive".to_string(),
            "- set size".to_string(),
            "- set history_size".to_string(),
            "- set end_condition".to_string(),
            "- set win_condition [Black|White]".to_string(),
            "- set revive".to_string(),
            "- 'help': list available commands".to_string(),
          ]
          .into_iter(),
        );
        Mode::CreateBoard
      }
      "help" => {
        self.send_response(
          vec![
            "- 'default': Create a default board".to_string(),
            "- 'load <filename>': Load a board from a file".to_string(),
            "- 'load_json': Load a board from json".to_string(),
            "- 'create': enter boardcreator".to_string(),
            "- 'help': list available commands".to_string(),
          ]
          .into_iter(),
        );
        stay
      }
      _ if msg.starts_with("load ") => {
        let filename = msg.trim_start_matches("load ").trim();
        match self.load_board(filename.to_string()) {
          Ok(_) => {
            self.send_response(
              vec![
                format!("Board loaded from {}", filename),
                "Next: Initialize pieces".to_string(),
                "Tip: Use 'show' at any point to view the board".to_string(),
              ]
              .into_iter(),
            );
            Mode::InitPieces
          }
          Err(e) => {
            self.send_response(
              vec![
                format!("Failed to load board: {}", e),
                "Try again or use 'default'".to_string(),
              ]
              .into_iter(),
            );
            stay
          }
        }
      }
      _ if msg.starts_with("load_json") => {
        if let Ok(json) = self.json.try_recv() {
          match self.load_board_json(json) {
            Ok(_) => {
              self.send_response(
                vec![
                  "Board loaded from json".to_string(),
                  "Next: Initialize pieces".to_string(),
                  "Tip: Use 'show' at any point to view the board".to_string(),
                ]
                .into_iter(),
              );
              Mode::InitPieces
            }
            Err(e) => {
              self.send_response(
                vec![
                  format!("Failed to load board: {}", e),
                  "Try again or use 'default'".to_string(),
                ]
                .into_iter(),
              );
              stay
            }
          }
        } else {
          self.send_response(
            vec![
              "No json was sent to load the board from.".to_string(),
              "Try again or use 'default'".to_string(),
            ]
            .into_iter(),
          );
          stay
        }
      }
      _ => {
        self.send_response(
          vec![
            "Invalid board initialization command.".to_string(),
            "Use 'default' or 'load <filename>'".to_string(),
          ]
          .into_iter(),
        );
        stay
      }
    }
  }

  fn create_piece(&mut self) {
    let mut piece = Piece::defualt_piece();
    while let Ok(msg) = self.receiver.recv() {
      match msg.as_str() {
        "help" => {
          self.send_response(
            vec![
              "- 'cancel': Go to piece placement".to_string(),
              "- 'export' to export a piece".to_string(),
              "- show id".to_string(),
              "- show name".to_string(),
              "- show deathrattle".to_string(),
              "- show battlecry".to_string(),
              "- show passive".to_string(),
              "- show onmove".to_string(),
              "- show aftermove".to_string(),
              "- show onkill".to_string(),
              "- show possiblemoves".to_string(),
              "- show movecondition".to_string(),
              "- show memory".to_string(),
              "- set id".to_string(),
              "- set name".to_string(),
              "- add deathrattle".to_string(),
              "- add battlecry".to_string(),
              "- add passive".to_string(),
              "- add onmove".to_string(),
              "- add aftermove".to_string(),
              "- add onkill".to_string(),
              "- add move".to_string(),
              "- set movecondition".to_string(),
              "- add memory".to_string(),
              "- 'help': list available commands".to_string(),
            ]
            .into_iter(),
          );
        }
        "cancel" => {
          self.send_response(vec!["Returning to piece placement".to_string()].into_iter());
          return;
        }
        "getstatus" => {
          self
            .sender
            .send(Some("piececreator".to_string()))
            .expect("Failed to send status");
          continue;
        }
        "export" => {
          if piece.id == "" || piece.name == "" {
            self.send_response(vec!["piece name and id must not be empty".to_string()].into_iter());
            continue;
          }
          let res = self.board.as_mut().unwrap().add_piece2(piece.clone());
          match res {
            Ok(()) => {
              self.send_response(
                vec!["exported a piece, going back to piece placement".to_string()].into_iter(),
              );
              return;
            }
            Err(err) => {
              self.send_response(vec![format!("Sth went wrong {}", err)].into_iter());
              continue;
            }
          }
        }
        "share" => {
          if let Ok(a) = serde_json::to_string(&piece) {
            self.send_response(vec![a].into_iter());
            if piece.id != "" && piece.name != "" {
              self.send_response(vec![(&piece).name.clone()].into_iter());
            }
          } else {
            self.send_response(
              vec!["can't create json from incomplete pawn".to_string()].into_iter(),
            );
          }
          continue;
        }
        "show id" => {
          self.send_response(vec![format!("{}", piece.id)].into_iter());
          continue;
        }
        "show name" => {
          self.send_response(vec![format!("{}", piece.name)].into_iter());
          continue;
        }
        "show deathrattle" => {
          self.send_response(vec![format!("{:?}", piece.deathrattle)].into_iter());
          continue;
        }
        "show battlecry" => {
          self.send_response(vec![format!("{:?}", piece.battlecry)].into_iter());
          continue;
        }
        "show passive" => {
          self.send_response(vec![format!("{:?}", piece.passive)].into_iter());
          continue;
        }
        "show onmove" => {
          self.send_response(vec![format!("{:?}", piece.onmove)].into_iter());
          continue;
        }
        "show aftermove" => {
          self.send_response(vec![format!("{:?}", piece.aftermove)].into_iter());
          continue;
        }
        "show onkill" => {
          self.send_response(vec![format!("{:?}", piece.onkill)].into_iter());
          continue;
        }
        "show possiblemoves" => {
          self.send_response(vec![format!("{:?}", piece.possiblemoves)].into_iter());
          continue;
        }
        "show movecondition" => {
          self.send_response(vec![format!("{:?}", piece.movecondition)].into_iter());
          continue;
        }
        "show memory" => {
          self.send_response(vec![format!("{:?}", piece.memory)].into_iter());
          continue;
        }
        _ if msg.starts_with("set id") => {
          let command = msg.trim_start_matches("set id").trim();
          let args: Vec<&str> = command.split_whitespace().collect();

          if args.len() != 1 {
            self.send_response(
              vec![format!(
                "wrong number of args expected 1 got {}",
                args.len()
              )]
              .into_iter(),
            );
            continue;
          }
          piece.id = args[0].to_string();
          self.send_response(vec![format!("set id to {}", args[0])].into_iter());
        }
        _ if msg.starts_with("set name") => {
          let command = msg.trim_start_matches("set name").trim();
          let args: Vec<&str> = command.split_whitespace().collect();

          if args.len() != 1 {
            self.send_response(
              vec![format!(
                "wrong number of args expected 1 got {}",
                args.len()
              )]
              .into_iter(),
            );
            continue;
          }
          piece.name = args[0].to_string();
          self.send_response(vec![format!("set name to {}", args[0])].into_iter());
        }
        "add deathrattle" => {
          let mut node2 = RuleState::Empty;
          let dr = self.create_rule(&mut node2, &mut vec![], false);
          if dr.is_none() {
            continue;
          }
          if piece.deathrattle.is_none() {
            piece.deathrattle = Some(vec![]);
          }
          piece.deathrattle.as_mut().unwrap().push(dr.unwrap());
        }
        "add battlecry" => {
          let mut node2 = RuleState::Empty;
          let bc = self.create_rule(&mut node2, &mut vec![], false);
          if bc.is_none() {
            continue;
          }
          if piece.battlecry.is_none() {
            piece.battlecry = Some(vec![]);
          }
          piece.battlecry.as_mut().unwrap().push(bc.unwrap());
        }
        "add passive" => {
          let mut node2 = RuleState::Empty;
          let p = self.create_rule(&mut node2, &mut vec![], false);
          if p.is_none() {
            continue;
          }
          if piece.passive.is_none() {
            piece.passive = Some(vec![]);
          }
          piece.passive.as_mut().unwrap().push(p.unwrap());
        }
        "add onmove" => {
          let mut node2 = RuleState::Empty;
          let om = self.create_rule(&mut node2, &mut vec![], false);
          if om.is_none() {
            continue;
          }
          if piece.onmove.is_none() {
            piece.onmove = Some(vec![]);
          }
          piece.onmove.as_mut().unwrap().push(om.unwrap());
        }
        "add aftermove" => {
          let mut node2 = RuleState::Empty;
          let am = self.create_rule(&mut node2, &mut vec![], false);
          if am.is_none() {
            continue;
          }
          if piece.aftermove.is_none() {
            piece.aftermove = Some(vec![]);
          }
          piece.aftermove.as_mut().unwrap().push(am.unwrap());
        }
        "add onkill" => {
          let mut node2 = RuleState::Empty;
          let ok = self.create_rule(&mut node2, &mut vec![], false);
          if ok.is_none() {
            continue;
          }
          if piece.onkill.is_none() {
            piece.onkill = Some(vec![]);
          }
          piece.onkill.as_mut().unwrap().push(ok.unwrap());
        }
        "add move" => {
          self.send_response(
            vec![
              "Welcome to MoveCreator".to_string(),
              "- show condition".to_string(),
              "- show consequences".to_string(),
              "- set condition".to_string(),
              "- add consequence".to_string(),
              "- cancel: go back to piece creation".to_string(),
              "- export: add move and go back to piece creation".to_string(),
              "- 'help': list available commands".to_string(),
              "Tip: Use 'show' at any point to view the board".to_string(),
            ]
            .into_iter(),
          );
          let mv = self.create_move();
          if mv.is_none() {
            continue;
          }

          piece.possiblemoves.push(mv.unwrap());
        }
        "set movecondition" => {
          let mut node2 = RuleState::Empty;
          let mc = self.create_rule(&mut node2, &mut vec![], false);
          piece.movecondition = mc;
        }
        "add memory" => {
          self.send_response(vec!["Enter memory name:".to_string()].into_iter());
          'outer: while let Ok(msg2) = self.receiver.recv() {
            let mut node2 = RuleState::Empty;
            let contents = self.create_rule(&mut node2, &mut vec![], true);
            if contents.is_none() {
              self.send_response(
                vec!["Invalid rule, would you like to try creating memory [y/n]".to_string()]
                  .into_iter(),
              );
              if self.tui {
                while let Ok(msg3) = self.receiver.recv() {
                  match msg3.as_str() {
                    "y" => {
                      self.send_response(vec!["Enter memory name:".to_string()].into_iter());
                      continue 'outer;
                    }
                    "n" => {
                      self.send_response(vec!["Canceled".to_string()].into_iter());
                      break 'outer;
                    }
                    _ => {
                      self.send_response(
                        vec!["Unrecognized command".to_string(), "Enter y/n.".to_string()]
                          .into_iter(),
                      );
                    }
                  }
                }
              } else {
                break;
              };
            }
            piece.memory.insert(msg2, contents.unwrap());
            self.send_response(vec!["Created memory".to_string()].into_iter());
            break;
          }
          //TODO test this
        }
        _ => {
          self.send_response(vec!["unknown command".to_string()].into_iter());
        }
      }
    }
  }

  fn handle_create_board(&mut self, msg: String) -> Mode {
    //WE START FROM Default board and then change stuff in it.
    //so we can do unwrap on board yippee
    let stay = Mode::CreateBoard;
    match msg.as_str() {
      "help" => {
        self.send_response(
          vec![
            "- 'continue': Go to piece placement".to_string(),
            "- show size".to_string(),
            "- show history_size".to_string(),
            "- show end_condition".to_string(),
            "- show win_condition".to_string(),
            "- show revive".to_string(),
            "- set size".to_string(),
            "- set history_size".to_string(),
            "- set end_condition".to_string(),
            "- set win_condition [Black|White]".to_string(),
            "- set revive".to_string(),
            "- 'help': list available commands".to_string(),
          ]
          .into_iter(),
        );
        stay
      }
      "show size" => {
        self.get_dimensions();
        stay
      }
      "show history_size" => {
        self.send_response(
          vec![format!("{}", self.board.as_ref().unwrap().history_size)].into_iter(),
        );
        stay
      }
      "show end_condition" => {
        self.send_response(
          vec![format!("{:?}", self.board.as_ref().unwrap().endcondition)].into_iter(),
        );
        stay
      }
      "show win_condition" => {
        self.send_response(
          vec![
            format!("{:?}", self.board.as_ref().unwrap().wincondition.0),
            format!("{:?}", self.board.as_ref().unwrap().wincondition.1),
          ]
          .into_iter(),
        );
        stay
      }
      "show revive" => {
        self.send_response(vec![format!("{:?}", self.board.as_ref().unwrap().revive)].into_iter());
        stay
      }
      "continue" => {
        //TODO add checks if valid board
        self.send_response(
          vec![
            "Board created successfully!".to_string(),
            "Next: Initialize pieces".to_string(),
            "- 'continue': Go to piece placement".to_string(),
            "- 'load <filename>: Load a piece from a file".to_string(),
            "- 'load_json': Load a piece from json".to_string(),
            "- 'create': enter piece creator".to_string(),
            "- 'help': list available commands".to_string(),
            "Tip: Use 'show' at any point to view the board".to_string(),
          ]
          .into_iter(),
        );
        Mode::InitPieces
      }
      _ if msg.starts_with("set size") => {
        let size = msg.trim_start_matches("set size").trim();
        let siz = self.board.as_ref().unwrap().parse_position(size);
        match siz {
          Some(s2) => {
            let mut s = s2;
            if s.0 == 0 {
              s.0 = 1;
            }
            if s.1 == 0 {
              s.1 = 1;
            }

            self.board.as_mut().unwrap().size = s;
            let mut board = Vec::with_capacity(s.0 as usize);
            for _i in 0..s.0 {
              let mut row = Vec::with_capacity(s.1 as usize);
              for _j in 0..s.1 {
                row.push(None);
              }
              board.push(row);
            }
            self.board.as_mut().unwrap().board = board.clone();
            self.send_response(vec!["successfully set size".to_string()].into_iter());
          }
          None => {
            self.send_response(
              vec![
                "wrong set size format".to_string(),
                "eg: set size 1,2".to_string(),
              ]
              .into_iter(),
            );
          }
        }
        stay
      }
      _ if msg.starts_with("set history_size") => {
        let size = msg.trim_start_matches("set history_size ").trim();
        let siz = size.parse::<u32>();
        match siz {
          Ok(s) => {
            self.board.as_mut().unwrap().history_size = s;
            self.send_response(vec!["successfully set history size".to_string()].into_iter());
          }
          Err(_) => {
            self.send_response(
              vec![
                "wrong set history_size format".to_string(),
                "eg: set history_size 2".to_string(),
              ]
              .into_iter(),
            );
          }
        }
        stay
      }
      "set end_condition" => {
        let mut node2 = RuleState::Empty;
        let ec = self.create_rule(&mut node2, &mut vec![], false);
        self.board.as_mut().unwrap().endcondition = ec;
        stay
      }
      "set win_condition White" => {
        let mut node2 = RuleState::Empty;
        let wc = self.create_rule(&mut node2, &mut vec![], false);
        if wc.is_none() {
          // self.send_response(vec!["Players must have a win_condition".to_string()].into_iter());
          return stay;
        }
        self.board.as_mut().unwrap().wincondition.0 = wc.unwrap();
        stay
      }
      "set win_condition Black" => {
        let mut node2 = RuleState::Empty;
        let wc = self.create_rule(&mut node2, &mut vec![], false);
        if wc.is_none() {
          // self.send_response(vec!["Players must have a win_condition".to_string()].into_iter());
          return stay;
        }
        self.board.as_mut().unwrap().wincondition.1 = wc.unwrap();
        stay
      }
      "set revive" => {
        let mut node2 = RuleState::Empty;
        let wc = self.create_rule(&mut node2, &mut vec![], false);
        if wc.is_none() {
          // self.send_response(vec!["Players must have a win_condition".to_string()].into_iter());
          return stay;
        }
        self.board.as_mut().unwrap().revive = wc.unwrap();
        stay
      }
      _ => {
        self.send_response(vec!["No such command".to_string(),"Available commands:\nshow [size|history_size|end_condition|win_condition|revive]\nset [size|history_size|end_condition|revive]\nset win_condition [Black|White]".to_string()].into_iter());
        stay
      }
    }
  }

  fn handle_init_pieces(&mut self, msg: String) -> Mode {
    let stay = Mode::InitPieces;
    match msg.as_str() {
      "help" => {
        self.send_response(
          vec![
            "- 'continue': Go to piece placement".to_string(),
            "- 'load <filename>: Load a piece from a file".to_string(),
            "- 'load_json': Load a piece from json".to_string(),
            "- 'create': enter piece creator".to_string(),
            "- 'help': list available commands".to_string(),
            "Tip: Use 'show' at any point to view the board".to_string(),
          ]
          .into_iter(),
        );
        stay
      }
      "continue" => {
        self.send_response(
          vec![
            "Pieces initialization ended.".to_string(),
            "Pieces are ready to be placed".to_string(),
            "Tip: Use 'show' at any point to view the board".to_string(),
          ]
          .into_iter(),
        );
        Mode::PlacePieces
      }
      "list" => {
        if let Some(a) = &self.board {
          self.send_response(a.pieces.clone().into_iter().map(|(a, _)| a));
        }
        stay
      }
      "create" => {
        self.send_response(
          vec![
            "Entering piece creator.".to_string(),
            "- 'cancel': Go to piece placement".to_string(),
            "- 'export' to export a piece".to_string(),
            "- show id".to_string(),
            "- show name".to_string(),
            "- show deathrattle".to_string(),
            "- show battlecry".to_string(),
            "- show passive".to_string(),
            "- show onmove".to_string(),
            "- show aftermove".to_string(),
            "- show onkill".to_string(),
            "- show possiblemoves".to_string(),
            "- show movecondition".to_string(),
            "- show memory".to_string(),
            "- set id".to_string(),
            "- set name".to_string(),
            "- add deathrattle".to_string(),
            "- add battlecry".to_string(),
            "- add passive".to_string(),
            "- add onmove".to_string(),
            "- add aftermove".to_string(),
            "- add onkill".to_string(),
            "- add move".to_string(),
            "- set movecondition".to_string(),
            "- add memory".to_string(),
            "- 'help': list available commands".to_string(),
          ]
          .into_iter(),
        );
        self.create_piece();
        stay
      }
      _ if msg.starts_with("load ") => {
        let filename = msg.trim_start_matches("load ").trim();
        match self.load_pieces(filename.to_string()) {
          Ok(_) => {
            self.send_response(vec![format!("Piece loaded from {}", filename)].into_iter());
            stay
          }
          Err(e) => {
            self.send_response(
              vec![
                format!("Failed to load piece: {}", e),
                "Try again or load another piece.".to_string(),
              ]
              .into_iter(),
            );
            stay
          }
        }
      }
      _ if msg.starts_with("load_json") => {
        if let Ok(json) = self.json.try_recv() {
          match self.load_pieces_json(json) {
            Ok(_) => {
              self.send_response(vec!["Piece loaded from json".to_string()].into_iter());
              stay
            }
            Err(e) => {
              self.send_response(
                vec![
                  format!("Failed to load piece: {}", e),
                  "Try again or load another piece.".to_string(),
                ]
                .into_iter(),
              );
              stay
            }
          }
        } else {
          self.send_response(
            vec![
              "No json was sent to load the piece from.".to_string(),
              "Try again or use 'default'".to_string(),
            ]
            .into_iter(),
          );
          stay
        }
      }
      _ => {
        self.send_response(
          vec![
            "Invalid pieces initialization command.".to_string(),
            "Use 'continue' or 'load <filename>'".to_string(),
            "Or use 'list' to get all loaded pieces.".to_string(),
          ]
          .into_iter(),
        );
        stay
      }
    }
  }

  fn handle_place_pieces(&mut self, msg: String) -> Mode {
    let stay = Mode::PlacePieces;
    match msg.as_str() {
      "help" => {
        self.send_response(vec![
                    "- 'continue': Go to the game".to_string(),
                    "- 'place  <piece> <position> <player>': place piece at given possition and assign owner".to_string(),
                    "- 'rest <piece> <player>': place piece at the cementary and assign owner".to_string(),
                    "- 'list': list available pieces".to_string(),
                    "- 'export': export the game in json format".to_string(),
                    "- 'help': list available commands".to_string(),
                    "Tip: Use 'show' at any point to view the board".to_string(),
                ].into_iter());
        stay
      }
      "show cementary white" => {
        self.send_response(
          vec![format!(
            "{:?}",
            self
              .board
              .as_ref()
              .unwrap()
              .cementaries
              .0
              .iter()
              .map(|x| x.name.clone())
              .collect::<Vec<String>>()
          )]
          .into_iter(),
        );
        stay
      }
      "show cementary black" => {
        self.send_response(
          vec![format!(
            "{:?}",
            self
              .board
              .as_ref()
              .unwrap()
              .cementaries
              .1
              .iter()
              .map(|x| x.name.clone())
              .collect::<Vec<String>>()
          )]
          .into_iter(),
        );
        stay
      }
      "continue" => {
        self.send_response(
          vec![
            "Pieces placing ended.".to_string(),
            "Game is ready to start!".to_string(),
            "Tip: Use 'show' to view the board".to_string(),
          ]
          .into_iter(),
        );
        Mode::Playing
      }
      "list" => {
        if let Some(a) = &self.board {
          self.send_response(a.pieces.clone().into_iter().map(|(a, _)| a));
        }
        stay
      }
      "export" => {
        if let Ok(a) = serde_json::to_string(self.board.as_ref().unwrap()) {
          self.send_response(vec![a].into_iter());
        }
        stay
      }
      _ if msg.starts_with("place ") => {
        let command = msg.trim_start_matches("place ").trim();
        let args: Vec<&str> = command.split_whitespace().collect();
        if args.len() != 3 {
          send_response_and_return!(
            self,
            "Wrong number of arguments.\n place [name] [position] [player]",
            stay
          );
        }
        let (piece_name, position, player) = (args[0], args[1], args[2]);

        let pos = match Board::get_valid_position(&self.board.as_mut().unwrap(), position) {
          Some(a) => a,
          None => {
            send_response_and_return!(
              self,
              "Wrong position.\n please enter command again with valid position",
              stay
            );
          }
        };

        if let Some(b) = &mut self.board {
          let piece_string = b.pieces.get(piece_name);
          let id = pos.0 * b.size.1 + pos.1;
          let mut piece = match match piece_string {
            Some(piece_str) => Piece::create(piece_str.to_string(), id as i32),
            None => {
              send_response_and_return!(self, "no such piece in board", stay);
            }
          } {
            Ok(p) => p,
            Err(e) => {
              send_response_and_return!(self, e, stay);
            }
          };

          let plres = Player::from_str(player);
          if plres.is_none() {
            send_response_and_return!(
              self,
              "Wrong player.\n please enter command again with valid player\n White/Black",
              stay
            );
          }
          let player = plres.unwrap();
          piece.owner = player;
          b.board[pos.0 as usize][pos.1 as usize] = Some(piece);
          self.send_response(vec![format!("Placed {}", piece_name)].into_iter());
        }
        stay
      }
      _ if msg.starts_with("rest ") => {
        let command = msg.trim_start_matches("rest ").trim();
        let args: Vec<&str> = command.split_whitespace().collect();
        if args.len() != 2 {
          send_response_and_return!(
            self,
            "Wrong number of arguments.\n rest [piece_name]  [player]",
            stay
          );
        }
        let (piece_name, player) = (args[0], args[1]);
        if let Some(b) = &mut self.board {
          let piece_string = b.pieces.get(piece_name);
          let plres = Player::from_str(player);
          if plres.is_none() {
            send_response_and_return!(
              self,
              "Wrong player.\n please enter command again with valid player\n White/Black",
              stay
            );
          }
          let player = plres.unwrap();

          let nm = (b.size.0 * b.size.1) as i32;
          let id = match player {
            Player::Black => nm + b.cementaries.1.len() as i32,
            Player::White => -(nm + b.cementaries.0.len() as i32),
          };

          let mut piece = match match piece_string {
            Some(piece_str) => Piece::create(piece_str.to_string(), id as i32),
            None => {
              send_response_and_return!(self, "no such piece in board", stay);
            }
          } {
            Ok(p) => p,
            Err(e) => {
              send_response_and_return!(self, e, stay);
            }
          };

          piece.owner = player;
          if player == Player::Black {
            b.cementaries.1.push(piece);
          } else {
            b.cementaries.0.push(piece);
          }
          self.send_response(vec![format!("Placed {} in cementary", piece_name)].into_iter());
        }
        stay
      }
      _ => {
        self.send_response(
          vec![
            "Invalid pieces place command.".to_string(),
            "Use 'continue' or 'place <piece> <position> <player>' or 'rest <piece> <player>'"
              .to_string(),
            "Or use 'list' to get all loaded pieces.".to_string(),
            "For list of available command use 'help'".to_string(),
          ]
          .into_iter(),
        );
        stay
      }
    }
  }

  fn handle_playing(&mut self, msg: String) -> Mode {
    let stay = Mode::Playing;
    match msg.as_str() {
      "help" => {
        self.send_response(vec![
                    "- 'move  <position1> <position2> <player>': move piece at 'position1' to 'position2'".to_string(),
                    "- 'revive <cementary index> <position>': revive piece from own cementary and plave it at 'position'".to_string(),
                    "- 'show cementary <color>': show cementary of 'color' player".to_string(),
                    "- 'end': end game".to_string(),
                    "- 'help': list available commands".to_string(),
                    "Tip: Use 'show' at any point to view the board".to_string(),
                ].into_iter());
        stay
      }
      "status" => {
        self.send_response(
          vec![
            "Current game status:".to_string(),
            //TODO: Add game status details
          ]
          .into_iter(),
        );
        stay
      }
      "show cementary white" => {
        self.send_response(
          vec![format!(
            "{:?}",
            self
              .board
              .as_ref()
              .unwrap()
              .cementaries
              .0
              .iter()
              .map(|x| x.name.clone())
              .collect::<Vec<String>>()
          )]
          .into_iter(),
        );
        stay
      }
      "show cementary black" => {
        self.send_response(
          vec![format!(
            "{:?}",
            self
              .board
              .as_ref()
              .unwrap()
              .cementaries
              .1
              .iter()
              .map(|x| x.name.clone())
              .collect::<Vec<String>>()
          )]
          .into_iter(),
        );
        stay
      }
      "end" => {
        self.send_response(
          vec![
            "Game ended.".to_string(),
            "Thank you for playing!".to_string(),
          ]
          .into_iter(),
        );
        Mode::Finished
      }
      _ if msg.starts_with("move ") => {
        let command = msg.trim_start_matches("move ").trim();
        let args: Vec<&str> = command.split_whitespace().collect();
        if args.len() != 2 {
          send_response_and_return!(
            self,
            "Wrong input.\n move [start position] [end position]",
            stay
          );
        }
        let (p1, p2) = (args[0], args[1]);
        let pos1 = Board::get_valid_position(&self.board.as_mut().unwrap(), p1);
        let pos2 = Board::get_valid_position(&self.board.as_mut().unwrap(), p2);

        if pos1.is_none() || pos2.is_none() {
          send_response_and_return!(
            self,
            "Wrong input.\n move [start position] [end position]",
            stay
          );
        }
        let pos1 = pos1.unwrap();
        let pos2 = pos2.unwrap();

        let moves = self
          .board
          .as_mut()
          .unwrap()
          .get_moves_to(pos1, pos2, 0)
          .clone();
        let plmove;
        match moves {
          None => {
            send_response_and_return!(
              self,
              format!("move from {:?} to {:?} can't be perfomed", pos1, pos2),
              stay
            );
          }
          Some(v) => {
            if v.len() == 1 {
              plmove = v[0].clone();
            } else {
              loop {
                self.send_response(
                  vec![format!("Pick a move [0-{}]", v.len() - 1).to_string()].into_iter(),
                );

                if let Ok(msg) = self.receiver.recv() {
                  if let Ok(num) = msg.parse::<u32>() {
                    if (num as usize) < v.len() {
                      plmove = v[num as usize].clone();
                      break;
                    }
                  }
                }
              }
            }
          }
        }
        let res = self.board.as_mut().unwrap().make_move(
          pos1,
          pos2,
          plmove,
          &self.sender,
          &self.receiver,
          true,
          false,
        );

        match res {
          Ok(v2) => self.match_game_state(v2, stay),
          Err(mm) => {
            send_response_and_return!(
              self,
              format!(
                "move from {:?} to {:?} can't be perfomed, {}",
                pos1, pos2, mm
              ),
              stay
            );
          }
        }
      }
      _ if msg.starts_with("revive ") => {
        let command = msg.trim_start_matches("revive ").trim();
        let args: Vec<&str> = command.split_whitespace().collect();
        if args.len() != 2 {
          send_response_and_return!(
            self,
            "Wrong input.\n revive [cementary index] [position]",
            stay
          );
        }

        let (i, p) = (args[0], args[1]);
        let pos = Board::get_valid_position(&self.board.as_mut().unwrap(), p);
        let idx = i.parse::<u32>();

        if pos.is_none() || idx.is_err() {
          send_response_and_return!(
            self,
            "Wrong index or position.\n revive [cementary index] [position]",
            stay
          );
        }

        let pos = pos.unwrap();
        let idx = idx.unwrap();

        let white = if self.board.as_ref().unwrap().current_player == Player::White {
          true
        } else {
          false
        };

        let res = self.board.as_mut().unwrap().revive_piece(
          white,
          idx,
          pos,
          &self.sender,
          &self.receiver,
          false,
        );

        match res {
          Ok(v2) => self.match_game_state(v2, stay),
          Err(mm) => {
            send_response_and_return!(
              self,
              format!(
                "revive from {:?} to {:?} can't be perfomed, {}",
                idx, pos, mm
              ),
              stay
            );
          }
        }
      }
      _ => {
        self.send_response(
          vec!["Unknown input, use 'help' to list available commands".to_string()].into_iter(),
        );
        stay
      }
    }
  }

  fn handle_finished(&mut self, msg: String) -> Mode {
    match msg.as_str() {
      "show" => {
        self.show_board();
        Mode::Finished
      }
      "restart" => {
        self.send_response(vec!["Restarting the game...".to_string()].into_iter());
        self.board = None;
        Mode::Start
      }
      _ => {
        self.send_response(
          vec![
            "Game is over.".to_string(),
            "Use 'restart' to start again or 'exit' to quit.".to_string(),
          ]
          .into_iter(),
        );
        Mode::Finished
      }
    }
  }

  fn match_game_state(&self, state: GameState, stay: Mode) -> Mode {
    match state {
      GameState::BlackWins => {
        send_response_and_return!(self, "Black wins", Mode::Finished);
      }
      GameState::WhiteWins => {
        send_response_and_return!(self, "White wins", Mode::Finished);
      }
      GameState::Draw => {
        send_response_and_return!(self, "It's a draw", Mode::Finished);
      }
      GameState::Continue => {
        let board_str = format!("{}", self.board.as_ref().unwrap());

        let curr = self.board.as_ref().unwrap().current_player.as_str();
        send_response_and_return!(
          self,
          format!("{}'s turn\nstate:\n{}", curr, board_str),
          stay
        );
      }
      GameState::Error(em) => {
        send_response_and_return!(
          self,
          format!("could not determine result. er: {}", em),
          stay
        );
      }
    }
  }
  ///stwórz domyślną pustą planszę
  fn new_default_board(&mut self) {
    self.board = Some(Board::new());
  }

  ///wczytaj planszę z pliku json
  fn load_board(&mut self, json_path: String) -> Result<(), String> {
    match Board::from_json({
      match std::fs::read_to_string(&json_path) {
        Ok(s) => s,
        Err(e) => {
          return Err(format!("{e}"));
        }
      }
    }) {
      Ok(board) => {
        self.board = Some(board);
        return Ok(());
      }
      Err(e) => Err(format!(
        "Could not create board from: {} because of {}",
        json_path, e
      )),
    }
  }

  ///wczytaj planszę z jsona
  fn load_board_json(&mut self, json: String) -> Result<(), String> {
    match Board::from_json(json) {
      Ok(board) => {
        self.board = Some(board);
        return Ok(());
      }
      Err(e) => Err(format!("Could not create board from json: {}", e)),
    }
  }

  ///wczytaj figurę z pliku json i dodaj do planszy
  fn load_pieces(&mut self, json_path: String) -> Result<(), String> {
    if let Some(b) = &mut self.board {
      b.add_piece({
        match std::fs::read_to_string(&json_path) {
          Ok(s) => s,
          Err(e) => {
            return Err(format!("{e}"));
          }
        }
      })?;
      return Ok(());
    }
    Err(format!("Could not load piece from: {}", json_path))
  }

  ///wczytaj figurę z jsona i dodaj do planszy
  fn load_pieces_json(&mut self, json: String) -> Result<(), String> {
    if let Some(b) = &mut self.board {
      b.add_piece(json)?;
      return Ok(());
    }
    Err(format!("Could not load piece from json"))
  }

  fn create_move(&self) -> Option<PMove> {
    let mut mv = PMove::default_move();

    while let Ok(msg) = self.receiver.recv() {
      match msg.as_str() {
        "help" => {
          self.send_response(
            vec![
              "- show condition".to_string(),
              "- show consequences".to_string(),
              "- set condition".to_string(),
              "- add consequence".to_string(),
              "- cancel: go back to piece creation".to_string(),
              "- export: add move and go back to piece creation".to_string(),
              "- 'help': list available commands".to_string(),
              "Tip: Use 'show' at any point to view the board".to_string(),
            ]
            .into_iter(),
          );
          continue;
        }
        "show condition" => {
          self.send_response(vec![format!("{:?}", mv.condition)].into_iter());
          continue;
        }
        "show consequences" => {
          self.send_response(vec![format!("{:?}", mv.consequences)].into_iter());
          continue;
        }
        "set condition" => {
          let mut node2 = RuleState::Empty;
          let cnd = self.create_rule(&mut node2, &mut vec![], false);
          if cnd.is_none() {
            continue;
          }
          mv.condition = Box::new(cnd.unwrap());
        }
        "getstatus" => {
          self
            .sender
            .send(Some("movecreator".to_string()))
            .expect("Failed to send status");
          continue;
        }
        "add consequence" => {
          let mut node2 = RuleState::Empty;
          let csq = self.create_rule(&mut node2, &mut vec![], false);
          if csq.is_none() {
            continue;
          }
          if mv.consequences.is_none() {
            mv.consequences = Some(vec![]);
          }
          mv.consequences
            .as_mut()
            .unwrap()
            .push(Box::new(csq.unwrap()));
        }
        "export" => {
          self.send_response(
            vec!["exporting move, going back to create piece".to_string()].into_iter(),
          );
          return Some(mv);
        }
        "cancel" => {
          self.send_response(
            vec!["movecreate cancelled, going back to create piece".to_string()].into_iter(),
          );
          return None;
        }
        _ => {
          self.send_response(
            vec!["unknown command, use 'help' to list available commands".to_string()].into_iter(),
          );
          continue;
        }
      }
    }
    None
  }

  fn create_boolean(&self, root: &mut RuleState, path: Vec<usize>) -> Option<RuleExpression> {
    self.send_response(
      vec![
        "Enter true or false".to_string(),
        "Enter 'status' to see rule preview".to_string(),
        "Enter 'cancel' to go back".to_string(),
      ]
      .into_iter(),
    );
    while let Ok(msg2) = self.receiver.recv() {
      match msg2.as_str() {
        "getstatus" => {
          self
            .sender
            .send(Some("rulecreator-bool".to_string()))
            .expect("Failed to send status");
          continue;
        }
        "true" => {
          add_node(
            root,
            path,
            0,
            RuleState::Closed(RuleExpression::Boolean(true)),
          );
          return Some(RuleExpression::Boolean(true));
        }
        "false" => {
          add_node(
            root,
            path,
            0,
            RuleState::Closed(RuleExpression::Boolean(false)),
          );
          return Some(RuleExpression::Boolean(false));
        }
        "cancel" => {
          // self.send_response(vec!["Canceled".to_string()].into_iter());
          return None;
        }
        "status" => {
          self.send_response(vec![print_rule_state(root.clone())].into_iter());
        }
        _ => {
          self.send_response(vec!["Invalid answer.Enter true or false".to_string()].into_iter());
        }
      }
    }
    None
  }

  fn create_integer(&self, root: &mut RuleState, path: Vec<usize>) -> Option<RuleExpression> {
    self.send_response(
      vec![
        "Enter integer value".to_string(),
        "Enter 'status' to see rule preview".to_string(),
        "Enter 'cancel' to go back".to_string(),
        "Enter 'help' to display this message".to_string(),
      ]
      .into_iter(),
    );
    while let Ok(msg2) = self.receiver.recv() {
      if msg2.as_str() == "getstatus" {
        self
          .sender
          .send(Some("rulecreator-integer".to_string()))
          .expect("Failed to send status");
        continue;
      }
      if msg2.as_str() == "cancel" {
        // self.send_response(vec!["Canceled".to_string()].into_iter());
        return None;
      }
      if msg2.as_str() == "help" {
        self.send_response(
          vec![
            "Enter integer value".to_string(),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
        continue;
      }
      if msg2.as_str() == "status" {
        self.send_response(vec![print_rule_state(root.clone())].into_iter());
        continue;
      }
      match msg2.parse::<i32>() {
        Ok(x) => {
          add_node(root, path, 0, RuleState::Closed(RuleExpression::Integer(x)));
          return Some(RuleExpression::Integer(x));
        }
        Err(_) => {
          self.send_response(vec!["Invalid integer. Enter integer value".to_string()].into_iter());
        }
      }
    }
    None
  }

  fn create_variable(&self, root: &mut RuleState, path: Vec<usize>) -> Option<RuleExpression> {
    self.send_response(
      vec![
        "Enter variable name".to_string(),
        "Enter 'status' to see rule preview".to_string(),
        "Enter 'cancel' to go back".to_string(),
        "Enter 'help' to display this message".to_string(),
      ]
      .into_iter(),
    );
    while let Ok(msg2) = self.receiver.recv() {
      if msg2.as_str() == "getstatus" {
        self
          .sender
          .send(Some("rulecreator-var".to_string()))
          .expect("Failed to send status");
        continue;
      }
      if msg2.as_str() == "help" {
        self.send_response(
          vec![
            "Enter variable name".to_string(),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
        continue;
      }
      if msg2.as_str() == "cancel" {
        // self.send_response(vec!["Canceled".to_string()].into_iter());
        return None;
      }
      if msg2.as_str() == "status" {
        self.send_response(vec![print_rule_state(root.clone())].into_iter());
        continue;
      }
      add_node(
        root,
        path,
        0,
        RuleState::Closed(RuleExpression::Variable(msg2.clone())),
      );
      return Some(RuleExpression::Variable(msg2));
    }
    None
  }

  fn create_apicall(&self, root: &mut RuleState, path: Vec<usize>) -> Option<RuleExpression> {
    let l1 = default_calls();
    self.send_response(
      vec![
        "To list apicalls enter 'list'".to_string(),
        "Enter 'status' to see rule preview".to_string(),
        "Enter 'cancel' to go back".to_string(),
        "Enter 'help' to display this message".to_string(),
        "Enter ApiCall name and arguments to create ApiCall.".to_string(),
      ]
      .into_iter(),
    );
    while let Ok(msg2) = self.receiver.recv() {
      match msg2.as_str() {
        "getstatus" => {
          self
            .sender
            .send(Some("rulecreator-api".to_string()))
            .expect("Failed to send status");
          continue;
        }
        "help" => {
          self.send_response(
            vec![
              "To list apicalls enter 'list'".to_string(),
              "Enter 'status' to see rule preview".to_string(),
              "Enter 'cancel' to go back".to_string(),
              "Enter 'help' to display this message".to_string(),
              "Enter ApiCall name and arguments to create ApiCall.".to_string(),
            ]
            .into_iter(),
          );
          continue;
        }
        "list" => {
          self.send_response(
            l1.iter()
              .map(|x| x.0.clone() + " " + &x.1 .1.clone() + "\n"),
          );
          continue;
        }
        "cancel" => {
          // self.send_response(vec!["Canceled".to_string()].into_iter());
          return None;
        }
        "status" => {
          self.send_response(vec![print_rule_state(root.clone())].into_iter());
          continue;
        }
        _ => {
          let command = msg2.trim();
          let args: Vec<&str> = command.split_whitespace().collect();

          if args.len() < 1 {
            self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
            continue;
          }

          let call = args[0];
          if l1.contains_key(call) {
            let temp = &l1[call];
            match temp.0 {
              //unlimited args
              u32::MAX => {}
              4294967294 => {
                //not 2 3 bc call name
                if args.len() != 3 && args.len() != 4 {
                  self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                  continue;
                }
              }
              _ => {
                if args.len() - 1 != temp.0 as usize {
                  self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                  continue;
                }
              }
            }
            add_node(
              root,
              path,
              0,
              RuleState::Closed(RuleExpression::ApiCall(
                call.to_string(),
                args
                  .clone()
                  .into_iter()
                  .skip(1)
                  .map(|x| x.to_string())
                  .collect(),
              )),
            );
            return Some(RuleExpression::ApiCall(
              call.to_string(),
              args.into_iter().skip(1).map(|x| x.to_string()).collect(),
            ));
          } else {
            self.send_response(vec!["No such call".to_string()].into_iter());
          }
        }
      }
    }
    None
  }

  fn create_pair_equals_diff_sum(
    &self,
    typ: i32,
    root: &mut RuleState,
    path: &mut Vec<usize>,
  ) -> Option<RuleExpression> {
    let name = match typ {
      0 => "Pair",
      1 => "Equals",
      2 => "Diff",
      _ => "Sum",
    };
    let ord = vec!["first", "second"];
    let mut element = 0;
    let mut contents: Vec<RuleExpression> = vec![];

    let mut additional_info = format!("Could not create {} component of {}", ord[element], name);
    let mut additional_info_f = false;
    add_node(
      root,
      path.clone(),
      0,
      RuleState::Open {
        name: name.to_string(),
        children: vec![],
        size: 2,
      },
    );
    while element < 2 {
      if additional_info_f {
        additional_info_f = false;
        self.send_response(
          vec![
            additional_info.clone(),
            format!(
              "Enter 'create' to create {} component of {}",
              ord[element], name
            ),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
      } else {
        self.send_response(
          vec![
            format!(
              "Enter 'create' to create {} component of {}",
              ord[element], name
            ),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
      }
      while let Ok(msg) = self.receiver.recv() {
        match msg.as_str() {
          "gettype" => {
            self.send_response(vec![name.to_string()].into_iter());
            continue;
          }
          "getstatus" => {
            self
              .sender
              .send(Some("rulecreator-pd".to_string()))
              .expect("Failed to send status");
            continue;
          }
          "help" => {
            self.send_response(
              vec![
                format!(
                  "Enter 'create' to create {} component of {}",
                  ord[element], name
                ),
                "Enter 'status' to see rule preview".to_string(),
                "Enter 'cancel' to go back".to_string(),
                "Enter 'help' to display this message".to_string(),
              ]
              .into_iter(),
            );
            continue;
          }
          "cancel" => {
            // self.send_response(vec!["Canceled".to_string()].into_iter());
            pop_node(root, path.clone(), 0);
            return None;
          }
          "status" => {
            self.send_response(vec![print_rule_state(root.clone())].into_iter());
            continue;
          }
          "create" => {
            path.push(element);

            if let Some(expr) = self.create_rule2(root, path) {
              contents.push(expr);
              element += 1;
            } else {
              additional_info = format!("Did not create {} component of {}", ord[element], name);
              additional_info_f = true;
            }
            path.pop();
            break;
          }
          _ => {
            self.send_response(vec!["Unrecognized command".to_string()].into_iter());
          }
        }
      }
    }
    let newrule = match typ {
      0 => RuleExpression::Pair(Box::new(contents[0].clone()), Box::new(contents[1].clone())),
      1 => RuleExpression::Equals(Box::new(contents[0].clone()), Box::new(contents[1].clone())),
      2 => RuleExpression::Diff(Box::new(contents[0].clone()), Box::new(contents[1].clone())),
      _ => RuleExpression::Sum(Box::new(contents[0].clone()), Box::new(contents[1].clone())),
    };
    swap_node(root, path.clone(), 0, RuleState::Closed(newrule.clone()));
    return Some(newrule);
  }

  fn create_first_second_not(
    &self,
    typ: i32,
    root: &mut RuleState,
    path: &mut Vec<usize>,
  ) -> Option<RuleExpression> {
    let name = if typ == 0 {
      "First"
    } else {
      if typ == 1 {
        "Second"
      } else {
        "Not"
      }
    };
    add_node(
      root,
      path.clone(),
      0,
      RuleState::Open {
        name: name.to_string(),
        children: vec![],
        size: 1,
      },
    );
    let mut additional_info = format!(" ");
    let mut additional_info_f = false;
    loop {
      if additional_info_f {
        additional_info_f = false;
        self.send_response(
          vec![
            additional_info.clone(),
            format!("Enter 'create' to create {}", name),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
      } else {
        self.send_response(
          vec![
            format!("Enter 'create' to create {}", name),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
      }
      while let Ok(msg) = self.receiver.recv() {
        match msg.as_str() {
          "gettype" => {
            self.send_response(vec![name.to_string()].into_iter());
            continue;
          }
          "getstatus" => {
            self
              .sender
              .send(Some("rulecreator-pd".to_string()))
              .expect("Failed to send status");
            continue;
          }
          "help" => {
            self.send_response(
              vec![
                format!("Enter 'create' to create {}", name),
                "Enter 'status' to see rule preview".to_string(),
                "Enter 'cancel' to go back".to_string(),
                "Enter 'help' to display this message".to_string(),
              ]
              .into_iter(),
            );
            continue;
          }
          "cancel" => {
            // self.send_response(vec!["Canceled".to_string()].into_iter());
            pop_node(root, path.clone(), 0);
            return None;
          }
          "status" => {
            self.send_response(vec![print_rule_state(root.clone())].into_iter());
            continue;
          }
          "create" => {
            path.push(0);
            if let Some(expr) = self.create_rule2(root, path) {
              let newrule = if typ == 0 {
                RuleExpression::First(Box::new(expr))
              } else {
                if typ == 1 {
                  RuleExpression::Second(Box::new(expr))
                } else {
                  RuleExpression::Not(Box::new(expr))
                }
              };
              path.pop();
              swap_node(root, path.clone(), 0, RuleState::Closed(newrule.clone()));

              return Some(newrule);
            } else {
              additional_info = format!("Did not create {}", name);
              additional_info_f = true;
              path.pop();
            }
            break;
          }
          _ => {
            self.send_response(vec!["Unrecognized command".to_string()].into_iter());
            break;
          }
        }
      }
    }
  }

  fn create_and_or(
    &self,
    isand: bool,
    root: &mut RuleState,
    path: &mut Vec<usize>,
  ) -> Option<RuleExpression> {
    let name = if isand { "And" } else { "Or" };
    let mut contents: Vec<RuleExpression> = vec![];

    let mut additional_info = "".to_string();
    let mut additional_info_f = false;
    add_node(
      root,
      path.clone(),
      0,
      RuleState::Open {
        name: name.to_string(),
        children: vec![],
        size: -1,
      },
    );
    loop {
      if additional_info_f {
        additional_info_f = false;
        self.send_response(
          vec![
            additional_info.clone(),
            format!("Current state of {}: {:?}", name, contents),
            format!("Enter 'create' to create next element for {}", name),
            format!("Enter 'close' to finish {}", name),
            format!("Enter 'list' to see current contents of {}", name),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
      } else {
        self.send_response(
          vec![
            format!("Current state of {}: {:?}", name, contents),
            format!("Enter 'create' to create next element for {}", name),
            format!("Enter 'close' to finish {}", name),
            format!("Enter 'list' to see current contents of {}", name),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
      }
      while let Ok(msg) = self.receiver.recv() {
        match msg.as_str() {
          "gettype" => {
            self.send_response(vec![name.to_string()].into_iter());
            continue;
          }
          "getstatus" => {
            self
              .sender
              .send(Some("rulecreator-pd".to_string()))
              .expect("Failed to send status");
            continue;
          }
          "help" => {
            self.send_response(
              vec![
                format!("Current state of {}: {:?}", name, contents),
                format!("Enter 'create' to create next element for {}", name),
                format!("Enter 'close' to finish {}", name),
                format!("Enter 'list' to see current contents of {}", name),
                "Enter 'status' to see rule preview".to_string(),
                "Enter 'cancel' to go back".to_string(),
                "Enter 'help' to display this message".to_string(),
              ]
              .into_iter(),
            );
          }
          "status" => {
            self.send_response(vec![print_rule_state(root.clone())].into_iter());
          }
          "cancel" => {
            // self.send_response(vec!["Canceled".to_string()].into_iter());
            pop_node(root, path.clone(), 0);
            return None;
          }
          "close" => {
            if contents.len() < 2 {
              additional_info_f = true;
              additional_info = "Contents length is lower than 2 could not close".to_string();
              break;
            }
            let newrule = if isand {
              RuleExpression::And(contents)
            } else {
              RuleExpression::Or(contents)
            };
            swap_node(root, path.clone(), 0, RuleState::Closed(newrule.clone()));
            return Some(newrule);
          }
          "list" => {
            self.send_response(
              vec![format!("Current state of {}: {:?}", name, contents)].into_iter(),
            );
          }
          "create" => {
            path.push(contents.len());

            if let Some(expr) = self.create_rule2(root, path) {
              contents.push(expr);
            } else {
              additional_info_f = true;
              additional_info = format!("Did not create next element in {}", name);
            }
            path.pop();
            break;
          }
          _ => {
            self.send_response(vec!["Unrecognized command".to_string()].into_iter());
          }
        }
      }
    }
  }

  fn create_if(&self, root: &mut RuleState, path: &mut Vec<usize>) -> Option<RuleExpression> {
    let elements = ["condition", "then-branch", "else-branch"];
    let mut element = 0;
    let mut contents: Vec<RuleExpression> = vec![];
    let mut additional_info = "".to_string();
    let mut additional_info_f = false;
    add_node(
      root,
      path.clone(),
      0,
      RuleState::Open {
        name: "If".to_string(),
        children: vec![],
        size: 3,
      },
    );
    while element < 3 {
      if additional_info_f {
        additional_info_f = false;
        self.send_response(
          vec![
            additional_info.clone(),
            format!("Enter 'create' to create {}", elements[element]),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
      } else {
        self.send_response(
          vec![
            format!("Enter 'create' to create {}", elements[element]),
            "Enter 'status' to see rule preview".to_string(),
            "Enter 'cancel' to go back".to_string(),
            "Enter 'help' to display this message".to_string(),
          ]
          .into_iter(),
        );
      }

      while let Ok(msg) = self.receiver.recv() {
        match msg.as_str() {
          "gettype" => {
            self.send_response(vec!["If".to_string()].into_iter());
            continue;
          }
          "getstatus" => {
            self
              .sender
              .send(Some("rulecreator-pd".to_string()))
              .expect("Failed to send status");
            continue;
          }
          "help" => {
            self.send_response(
              vec![
                format!("Enter 'create' to create {}", elements[element]),
                "Enter 'status' to see rule preview".to_string(),
                "Enter 'cancel' to go back".to_string(),
                "Enter 'help' to display this message".to_string(),
              ]
              .into_iter(),
            );
            continue;
          }
          "cancel" => {
            // self.send_response(vec!["Canceled".to_string()].into_iter());
            pop_node(root, path.clone(), 0);
            return None;
          }
          "status" => {
            self.send_response(vec![print_rule_state(root.clone())].into_iter());
          }
          "create" => {
            path.push(element);
            if let Some(expr) = self.create_rule2(root, path) {
              contents.push(expr);
              element += 1;
            } else {
              additional_info_f = true;
              additional_info = format!("Did not create {}", elements[element]);
            }
            path.pop();
            break;
          }
          _ => {
            self.send_response(vec!["Unrecognized command".to_string()].into_iter());
          }
        }
      }
    }
    swap_node(
      root,
      path.clone(),
      0,
      RuleState::Closed(RuleExpression::If(
        Box::new(contents[0].clone()),
        Box::new(contents[1].clone()),
        Box::new(contents[2].clone()),
      )),
    );
    return Some(RuleExpression::If(
      Box::new(contents[0].clone()),
      Box::new(contents[1].clone()),
      Box::new(contents[2].clone()),
    ));
  }

  fn create_rule2(&self, root: &mut RuleState, path: &mut Vec<usize>) -> Option<RuleExpression> {
    self.send_response(
      vec![
        "Pick an expression type".to_string(),
        "1. Void".to_string(),
        "2. Boolean".to_string(),
        "3. Integer".to_string(),
        "4. Variable".to_string(),
        "5. ApiCall".to_string(),
        "6. Pair".to_string(),
        "7. First".to_string(),
        "8. Second".to_string(),
        "9. Diff".to_string(),
        "10. And".to_string(),
        "11. Or".to_string(),
        "12. Not".to_string(),
        "13. If".to_string(),
        "14. Equals".to_string(),
        "Enter 'cancel' to go back".to_string(),
        "Enter 'state' to see rule preview".to_string(),
        "Enter 'help' to display this message".to_string(),
      ]
      .into_iter(),
    );

    while let Ok(msg) = self.receiver.recv() {
      match msg.as_str() {
        "getstatus" => {
          self
            .sender
            .send(Some("rulecreator".to_string()))
            .expect("Failed to send status");
        }
        "help" => {
          self.send_response(
            vec![
              "Pick an expression type".to_string(),
              "1. Void".to_string(),
              "2. Boolean".to_string(),
              "3. Integer".to_string(),
              "4. Variable".to_string(),
              "5. ApiCall".to_string(),
              "6. Pair".to_string(),
              "7. First".to_string(),
              "8. Second".to_string(),
              "9. Diff".to_string(),
              "10. And".to_string(),
              "11. Or".to_string(),
              "12. Not".to_string(),
              "13. If".to_string(),
              "14. Equals".to_string(),
              "Enter 'cancel' to go back".to_string(),
              "Enter 'state' to see rule preview".to_string(),
              "Enter 'help' to display this message".to_string(),
            ]
            .into_iter(),
          );
        }
        "cancel" => {
          // self.send_response(vec!["Canceled.".to_string()].into_iter());
          return None;
        }
        "status" => {
          self.send_response(vec![print_rule_state(root.clone())].into_iter());
        }
        "1" => {
          add_node(
            root,
            path.clone(),
            0,
            RuleState::Closed(RuleExpression::Void),
          );
          return Some(RuleExpression::Void);
        }
        "2" => {
          return self.create_boolean(root, path.clone());
        }
        "3" => {
          return self.create_integer(root, path.clone());
        }
        "4" => {
          return self.create_variable(root, path.clone());
        }
        "5" => {
          return self.create_apicall(root, path.clone());
        }
        "6" => {
          return self.create_pair_equals_diff_sum(0, root, path);
        }
        "7" => {
          return self.create_first_second_not(0, root, path);
        }
        "8" => {
          return self.create_first_second_not(1, root, path);
        }
        "9" => {
          return self.create_pair_equals_diff_sum(2, root, path);
        }
        "10" => {
          return self.create_and_or(true, root, path);
        }
        "11" => {
          return self.create_and_or(false, root, path);
        }
        "12" => {
          return self.create_first_second_not(2, root, path);
        }
        "13" => {
          return self.create_if(root, path);
        }
        "14" => {
          return self.create_pair_equals_diff_sum(1, root, path);
        }
        "15" => {
          return self.create_pair_equals_diff_sum(3, root, path);
        }
        _ => {
          self.send_response(
            vec!["Unrecognized option, please pick one of the available options.".to_string()]
              .into_iter(),
          );
        }
      };
    }
    None
  }

  fn create_rule(
    &self,
    root: &mut RuleState,
    path: &mut Vec<usize>,
    silent: bool,
  ) -> Option<RuleExpression> {
    let ret = self.create_rule2(root, path);
    if !silent {
      if ret.is_some() {
        self.send_response(vec!["Rule created.".to_string()].into_iter());
      } else {
        self.send_response(vec!["Failed to create a rule.".to_string()].into_iter());
      }
    }
    return ret;
  }
}
