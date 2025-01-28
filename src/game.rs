use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::board::{Board, default_calls};
use crate::player::Player;
use crate::{PMove, Piece, RuleExpression};
use crate::GameState;

macro_rules! send_response_and_return {
    ($self:ident, $msg:expr, $stay:expr) => {{
        $self.send_response(vec![$msg.to_string()].into_iter());
        return $stay;
    }};
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

impl Mode{
    pub fn to_string(&self) -> String{
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

    json: Receiver<String>
}

impl Game {
    ///uruchamia wątek gry
    pub fn start_game(rx: Receiver<String>, tx: Sender<Option<String>>) -> (thread::JoinHandle<()>, Sender<String>) {
        let (tx_json, rx_json) = mpsc::channel();
        (thread::spawn(move || {
            let mut game = Game::new(rx, tx, rx_json);

            game.game_loop();
        }), tx_json)
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

            if msg == String::from("getstatus"){
                self.sender.send(Some(self.current_mode.to_string())).expect("Failed to send status");
                continue;
            }

            if msg == String::from("getdimensions"){
                self.get_dimensions();
                continue;
            }

            if msg == String::from("currentwhite"){
                self.currentwhite();
                continue;
            }

            if msg.starts_with("verifypiece"){
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
    pub fn new(rx: Receiver<String>, tx: Sender<Option<String>>, rx_json: Receiver<String>) -> Self {
        Self {
            board: None,
            receiver: rx,
            sender: tx,
            current_mode: Mode::Start,
            json: rx_json
        }
    }

    fn send_response<I>(&self, strings: I)
    where
    I: Iterator<Item = String>,
    {
        for str in strings {
            self.sender.send(Some(str)).expect("Failed to send response");
        }
        self.sender.send(None).expect("Failed to send end of response");
    }

    fn show_board(&self) {
        match &self.board {
            Some(board) => {
                let board_str = format!("{}", board);

                let board_lines = board_str.lines()
                .map(|line| line.to_string())
                .collect::<Vec<String>>();

                self.send_response(board_lines.into_iter());
            },
            None => {
                self.send_response(vec![
                    "No board initialized yet.".to_string()
                ].into_iter());
            }
        }
    }

    fn get_dimensions(&self){
        match &self.board{
            Some(board) => {
                let size = format!("{},{}",board.size.0, board.size.1).to_string();
                let v = vec![size];
                self.send_response(v.into_iter());
            }
            None => {
                self.send_response(vec![
                    "No board initialized yet.".to_string()
                ].into_iter());
            }
        }
    }

    fn verifypiece(&mut self, msg: String) {
        let json = msg.trim_start_matches("verifypiece ").trim();
        let args: Vec<&str> = json.split_whitespace().collect();
        if args.len()!=1{
            self.send_response(vec!["Wrong input.\n verifypiece [json]".to_string()].into_iter());
        }

        match self.load_pieces_json(json.to_string()) {
            Ok(_) => {
                self.send_response(vec![
                    "Piece loaded from json".to_string()
                ].into_iter())
            }
            Err(e) => {
                self.send_response(vec![
                    format!("Failed to load piece: {}", e),
                        "Try again or load another piece.".to_string(),
                ].into_iter())
            }
        }
    }

    fn currentwhite(&self){
        let ret = if self.board.as_ref().unwrap().current_player==Player::White {"White"} else {"Black"};
        self.send_response(vec![ret.to_string()].into_iter());
    }

    fn handle_start(&self, _msg: String) -> Mode {
        self.send_response(vec![
            "Welcome to the Board Games!".to_string(),
            "Choose how to initialize the board:".to_string(),
            "- 'default': Create a default board".to_string(),
            "- 'load <filename>': Load a board from a file".to_string(),
            "- 'load_json': Load a board from json".to_string(),
        ].into_iter());
        Mode::InitBoard
    }

    /// Handle board initialization state
    fn handle_init_board(&mut self, msg: String) -> Mode {
        let stay = Mode::InitBoard;
        match msg.as_str() {
            "default" => {
                self.new_default_board();
                self.send_response(vec![
                    "Default board created successfully!".to_string(),
                    "Next: Initialize pieces".to_string(),
                    "- 'continue': Go to piece placement".to_string(),
                    "- 'load <filename>: Load a piece from a file".to_string(),
                    "- 'load_json': Load a piece from json".to_string(),
                    "Tip: Use 'show' at any point to view the board".to_string(),
                ].into_iter());
                Mode::InitPieces
            }
            "create" => {
                self.new_default_board();
                self.send_response(vec![
                    "Default board created successfully!".to_string(),
                    "Next: Edit board".to_string(),
                    "- 'continue': Go to piece placement".to_string(),
                    "- show size".to_string(),
                    "- show history_size".to_string(),
                    "- show end_condition".to_string(),
                    "- show win_condition".to_string(),
                    "- set size".to_string(),
                    "- set history_size".to_string(),
                    "- set end_condition".to_string(),
                    "- set win_condition [Black|White]".to_string(),
                ].into_iter());
                Mode::CreateBoard
            }
            _ if msg.starts_with("load ") => {
                let filename = msg.trim_start_matches("load ").trim();
                match self.load_board(filename.to_string()) {
                    Ok(_) => {
                        self.send_response(vec![
                            format!("Board loaded from {}", filename),
                                    "Next: Initialize pieces".to_string(),
                                    "Tip: Use 'show' at any point to view the board".to_string(),
                        ].into_iter());
                        Mode::InitPieces
                    }
                    Err(e) => {
                        self.send_response(vec![
                            format!("Failed to load board: {}", e),
                                    "Try again or use 'default'".to_string(),
                        ].into_iter());
                        stay
                    }
                }
            },
            _ if msg.starts_with("load_json") => {
                if let Ok(json) = self.json.try_recv() {
                    match self.load_board_json(json) {
                        Ok(_) => {
                            self.send_response(vec![
                                "Board loaded from json".to_string(),
                                "Next: Initialize pieces".to_string(),
                                "Tip: Use 'show' at any point to view the board".to_string(),
                            ].into_iter());
                            Mode::Playing
                        }
                        Err(e) => {
                            self.send_response(vec![
                                format!("Failed to load board: {}", e),
                                "Try again or use 'default'".to_string(),
                            ].into_iter());
                            stay
                        }
                    }
                }
                else {
                    self.send_response(vec![
                        "No json was sent to load the board from.".to_string(),
                        "Try again or use 'default'".to_string(),
                    ].into_iter());
                    stay
                }
            }
            _ => {
                self.send_response(vec![
                    "Invalid board initialization command.".to_string(),
                    "Use 'default' or 'load <filename>'".to_string(),
                ].into_iter());
                stay
            }
        }
    }

    fn create_piece(&mut self) {
        let mut piece = Piece::defualt_piece();
        while let Ok(msg) = self.receiver.recv() {
            match msg.as_str(){
                "cancel" => {
                    self.send_response(vec!["Returning to piece placement".to_string()].into_iter());
                    return;
                },
                "getstatus" => {
                    self.sender.send(Some("piececreator".to_string())).expect("Failed to send status");
                    continue;
                }
                "export"=>{
                    if piece.id == "" || piece.name == ""{
                        self.send_response(vec!["piece name and id must not be empty".to_string()].into_iter());
                        continue;
                    }
                    let res = self.board.as_mut().unwrap().add_piece2(piece.clone());
                    match res{
                        Ok(()) =>{
                            self.send_response(vec!["exported a piece, going back to piece placement".to_string()].into_iter());
                            return;
                        }
                        Err(err) =>{
                            self.send_response(vec![format!("Sth went wrong {}",err)].into_iter());
                            continue;
                        }
                    }
                },
                "show id" =>{
                    self.send_response(vec![format!("{}",piece.id)].into_iter());
                    continue;
                },
                "show name" =>{
                    self.send_response(vec![format!("{}",piece.name)].into_iter());
                    continue;
                },
                "show deathrattle" =>{
                    self.send_response(vec![format!("{:?}",piece.deathrattle)].into_iter());
                    continue;
                },
                "show battlecry" =>{
                    self.send_response(vec![format!("{:?}",piece.battlecry)].into_iter());
                    continue;
                },
                "show passive" =>{
                    self.send_response(vec![format!("{:?}",piece.passive)].into_iter());
                    continue;
                },
                "show onmove" =>{
                    self.send_response(vec![format!("{:?}",piece.onmove)].into_iter());
                    continue;
                },
                "show aftermove" =>{
                    self.send_response(vec![format!("{:?}",piece.aftermove)].into_iter());
                    continue;
                },
                "show onkill" =>{
                    self.send_response(vec![format!("{:?}",piece.onkill)].into_iter());
                    continue;
                },
                "show possiblemoves" =>{
                    self.send_response(vec![format!("{:?}",piece.possiblemoves)].into_iter());
                    continue;
                },
                "show movecondition" =>{
                    self.send_response(vec![format!("{:?}",piece.movecondition)].into_iter());
                    continue;
                },
                "show memory" =>{
                    self.send_response(vec![format!("{:?}",piece.memory)].into_iter());
                    continue;
                },
                _ if msg.starts_with("set id") => {
                    println!("ooooooooo");
                    let command = msg.trim_start_matches("set id").trim();
                    let args: Vec<&str> = command.split_whitespace().collect();

                    if args.len()!=1{
                        self.send_response(vec![format!("wrong number of args expected 1 got {}", args.len())].into_iter());
                        continue;
                    }
                    piece.id=args[0].to_string();
                    self.send_response(vec![format!("set id to {}",args[0])].into_iter());
                },
                _ if msg.starts_with("set name") =>{
                    let command = msg.trim_start_matches("set name").trim();
                    let args: Vec<&str> = command.split_whitespace().collect();

                    if args.len()!=1{
                        self.send_response(vec![format!("wrong number of args expected 1 got {}", args.len())].into_iter());
                        continue;
                    }
                    piece.name=args[0].to_string();
                    self.send_response(vec![format!("set name to {}",args[0])].into_iter());
                },
                "add deathrattle" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let dr = self.create_rule();
                    if dr.is_none(){
                        continue;
                    }
                    if piece.deathrattle.is_none(){
                        piece.deathrattle = Some(vec![]);
                    }
                    piece.deathrattle.as_mut().unwrap().push(dr.unwrap());
                },
                "add battlecry" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let bc = self.create_rule();
                    if bc.is_none(){
                        continue;
                    }
                    if piece.battlecry.is_none(){
                        piece.battlecry = Some(vec![]);
                    }
                    piece.battlecry.as_mut().unwrap().push(bc.unwrap());
                },
                "add passive" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let p = self.create_rule();
                    if p.is_none(){
                        continue;
                    }
                    if piece.passive.is_none(){
                        piece.passive = Some(vec![]);
                    }
                    piece.passive.as_mut().unwrap().push(p.unwrap());
                },
                "add onmove" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let om = self.create_rule();
                    if om.is_none(){
                        continue;
                    }
                    if piece.onmove.is_none(){
                        piece.onmove = Some(vec![]);
                    }
                    piece.onmove.as_mut().unwrap().push(om.unwrap());
                },
                "add aftermove" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let am = self.create_rule();
                    if am.is_none(){
                        continue;
                    }
                    if piece.aftermove.is_none(){
                        piece.aftermove = Some(vec![]);
                    }
                    piece.aftermove.as_mut().unwrap().push(am.unwrap());
                },
                "add onkill" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let ok = self.create_rule();
                    if ok.is_none(){
                        continue;
                    }
                    if piece.onkill.is_none(){
                        piece.onkill = Some(vec![]);
                    }
                    piece.onkill.as_mut().unwrap().push(ok.unwrap());
                },
                "add move" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let mv = self.create_move();
                    if mv.is_none(){
                        continue;
                    }

                    piece.possiblemoves.push(mv.unwrap());
                },
                "set movecondition" => {
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let mc = self.create_rule();
                    piece.movecondition=mc;
                },
                "add memory" =>{

                }
                _ =>{
                    self.send_response(vec!["unknown command".to_string()].into_iter());
                }
            }
        }
    }

    fn handle_create_board(&mut self, msg: String) -> Mode{
        //WE START FROM Default board and then change stuff in it.
        //so we can do unwrap on board yippee
        let stay = Mode::CreateBoard;
        match msg.as_str(){
            "show size" =>{
                self.get_dimensions();
                stay
            },
            "show history_size" =>{
                self.send_response(vec![format!("{}",self.board.as_ref().unwrap().history_size)].into_iter());
                stay
            },
            "show end_condition" =>{
                self.send_response(vec![format!("{:?}",self.board.as_ref().unwrap().endcondition)].into_iter());
                stay
            },
            "show win_condition" =>{
                self.send_response(vec![format!("{:?}",self.board.as_ref().unwrap().wincondition.0), format!("{:?}",self.board.as_ref().unwrap().wincondition.1)].into_iter());
                stay
            },
            "continue" => {
                //TODO add checks if valid board
                self.send_response(vec![
                    "Board created successfully!".to_string(),
                    "Next: Initialize pieces".to_string(),
                    "- 'continue': Go to piece placement".to_string(),
                    "- 'load <filename>: Load a piece from a file".to_string(),
                    "- 'load_json': Load a piece from json".to_string(),
                    "Tip: Use 'show' at any point to view the board".to_string(),
                ].into_iter());
                Mode::InitPieces
            },
            _ if msg.starts_with("set size") => {
                let size = msg.trim_start_matches("set size").trim();
                let siz = self.board.as_ref().unwrap().parse_position(size);
                match siz{
                    Some(s) =>{
                        self.board.as_mut().unwrap().size=s;
                        let mut board = Vec::with_capacity(s.0 as usize);
                        for _i in 0..s.0{
                            let mut row = Vec::with_capacity(s.1 as usize);
                            for _j in 0..s.1 {
                                row.push(None);
                            }
                            board.push(row);
                        }
                        self.board.as_mut().unwrap().board=board.clone();
                        self.send_response(vec!["successfully set size".to_string()].into_iter());
                    }
                    None =>{
                        self.send_response(vec!["wrong set size format".to_string(), "eg: set size 1,2".to_string()].into_iter());
                    }
                }
                stay
            },
            _ if msg.starts_with("set history_size") => {
                let size = msg.trim_start_matches("set history_size ").trim();
                let siz = size.parse::<u32>();
                match siz{
                    Ok(s) =>{
                        self.board.as_mut().unwrap().history_size=s;
                        self.send_response(vec!["successfully set history size".to_string()].into_iter());
                    }
                    Err(_) =>{
                        self.send_response(vec!["wrong set history_size format".to_string(), "eg: set history_size 2".to_string()].into_iter());
                    }
                }
                stay
            },
            "set end_condition"=>{
                self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                let ec = self.create_rule();
                self.board.as_mut().unwrap().endcondition=ec;
                stay
            },
            "set win_condition White" =>{
                self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                let wc = self.create_rule();
                if wc.is_none(){
                   // self.send_response(vec!["Players must have a win_condition".to_string()].into_iter());
                    return stay;
                }
                self.board.as_mut().unwrap().wincondition.0=wc.unwrap();
                stay
            },
            "set win_condition Black" =>{
                self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                let wc = self.create_rule();
                if wc.is_none(){
                   // self.send_response(vec!["Players must have a win_condition".to_string()].into_iter());
                    return stay;
                }
                self.board.as_mut().unwrap().wincondition.1=wc.unwrap();
                stay
            },
            _ =>{
                self.send_response(vec!["No such command".to_string(),"Available commands:\nshow [size|history_size|end_condition|win_condition]\nset [size|history_size|end_condition]\nset win_condition [Black|White]".to_string()].into_iter());
                stay
            }
        }
    }

    fn handle_init_pieces(&mut self, msg: String) -> Mode {
        let stay = Mode::InitPieces;
        match msg.as_str() {
            "continue" => {
                self.send_response(vec![
                    "Pieces initialization ended.".to_string(),
                    "Pieces are ready to be placed".to_string(),
                    "Tip: Use 'show' at any point to view the board".to_string(),
                ].into_iter());
                Mode::PlacePieces
            },
            "list" => {
                if let Some(a) = &self.board {
                    self.send_response(a.pieces.clone().into_iter().map(|(a,_)| a));
                }
                stay
            }
            "create" => {
                self.send_response(vec![
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
                ].into_iter());
                self.create_piece();
                stay
            }
            _ if msg.starts_with("load ") => {
                let filename = msg.trim_start_matches("load ").trim();
                match self.load_pieces(filename.to_string()) {
                    Ok(_) => {
                        self.send_response(vec![
                            format!("Piece loaded from {}", filename)
                        ].into_iter());
                        stay
                    }
                    Err(e) => {
                        self.send_response(vec![
                            format!("Failed to load piece: {}", e),
                                    "Try again or load another piece.".to_string(),
                        ].into_iter());
                        stay
                    }
                }
            }
            _ if msg.starts_with("load_json") => {
                if let Ok(json) = self.json.try_recv() {
                    match self.load_pieces_json(json) {
                        Ok(_) => {
                            self.send_response(vec![
                                "Piece loaded from json".to_string()
                            ].into_iter());
                            stay
                        }
                        Err(e) => {
                            self.send_response(vec![
                                format!("Failed to load piece: {}", e),
                                    "Try again or load another piece.".to_string(),
                            ].into_iter());
                            stay
                        }
                    }
                }
                else {
                    self.send_response(vec![
                        "No json was sent to load the piece from.".to_string(),
                        "Try again or use 'default'".to_string(),
                    ].into_iter());
                    stay
                }
            }
            _ => {
                self.send_response(vec![
                    "Invalid pieces initialization command.".to_string(),
                    "Use 'continue' or 'load <filename>'".to_string(),
                    "Or use 'list' to get all loaded pieces.".to_string()
                ].into_iter());
                stay
            }
        }
    }

    fn handle_place_pieces(&mut self, msg: String) -> Mode {
        let stay = Mode::PlacePieces;
        match msg.as_str() {
            "continue" => {
                self.send_response(vec![
                    "Pieces placing ended.".to_string(),
                    "Game is ready to start!".to_string(),
                    "Tip: Use 'show' to view the board".to_string(),
                ].into_iter());
                Mode::Playing
            },
            "list" => {
                if let Some(a) = &self.board {
                    self.send_response(a.pieces.clone().into_iter().map(|(a,_)| a));
                }
                stay
            },
            "export" =>{
                if let Ok(a)= serde_json::to_string(self.board.as_ref().unwrap()){
                    self.send_response(vec![a].into_iter());
                }
                stay
            }
            _ if msg.starts_with("place ") => {
                let command = msg.trim_start_matches("place ").trim();
                let args: Vec<&str> = command.split_whitespace().collect();
                if args.len()!=3{
                    send_response_and_return!(self,"Wrong number of arguments.\n place [name] [position] [player]",stay);
                }
                let (piece_name, position, player) = (args[0], args[1], args[2]);

                let pos = match Board::parse_position(&self.board.as_mut().unwrap(), position)
                {
                    Some(a) => a,
                    None => {
                        send_response_and_return!(self,"Wrong position.\n please enter command again with valid position", stay);
                    }
                };
                if let Some(b) = &mut self.board {
                    let piece_string = b.pieces.get(piece_name);
                    let mut piece = match match piece_string {
                        Some(piece_str) => {
                            Piece::new(piece_str.to_string())
                        }
                        None => {
                            send_response_and_return!(self, "no such piece in board", stay);
                        }
                    } {
                        Ok(p) => p,
                        Err(e) => {
                            send_response_and_return!(self,e,stay);
                        }
                    };

                    let plres = Player::from_str(player);
                    if plres.is_none(){
                        send_response_and_return!(self,"Wrong player.\n please enter command again with valid player\n White/Black", stay);
                    }
                    let player = plres.unwrap();
                    piece.owner = player;
                    b.board[pos.0 as usize][pos.1 as usize] = Some(piece);
                    self.send_response(vec![format!("Placed {}",piece_name)].into_iter());
                }
                stay
            }
            _ => {
                self.send_response(vec![
                    "Invalid pieces place command.".to_string(),
                    "Use 'continue' or 'place <piece> <position> <player>'".to_string(),
                    "Or use 'list' to get all loaded pieces.".to_string()
                ].into_iter());
                stay
            }
        }
    }

    fn handle_playing(&mut self, msg: String) -> Mode {
        let stay = Mode::Playing;
        match msg.as_str() {
            "status" => {
                self.send_response(vec![
                    "Current game status:".to_string(),
                    //TODO: Add game status details
                ].into_iter());
                stay
            }
            "show cementary white" =>{
                self.send_response(vec![format!("{:?}",self.board.as_ref().unwrap().cementaries.0.iter().map(|x| x.name.clone()).collect::<Vec<String>>())].into_iter());
                stay
            }
            "show cementary black" =>{
                self.send_response(vec![format!("{:?}",self.board.as_ref().unwrap().cementaries.1.iter().map(|x| x.name.clone()).collect::<Vec<String>>())].into_iter());
                stay
            }
            "end" => {
                self.send_response(vec![
                    "Game ended.".to_string(),
                    "Thank you for playing!".to_string(),
                ].into_iter());
                Mode::Finished
            }
            _ if msg.starts_with("move ")  => {
                let command = msg.trim_start_matches("move ").trim();
                let args: Vec<&str> = command.split_whitespace().collect();
                if args.len()!=2{
                    send_response_and_return!(self, "Wrong input.\n move [start position] [end position]", stay);
                }
                let (p1, p2) = (args[0], args[1]);
                let pos1 = Board::parse_position(&self.board.as_mut().unwrap(), p1);
                let pos2 = Board::parse_position(&self.board.as_mut().unwrap(), p2);

                if pos1.is_none() || pos2.is_none(){
                    send_response_and_return!(self, "Wrong input.\n move [start position] [end position]", stay);
                }

                let pos1 = pos1.unwrap(); let pos2 = pos2.unwrap();

                let moves = self.board.as_mut().unwrap().get_moves_to(pos1, pos2, 0).clone();
                let plmove;
                match moves{
                    None =>{
                        send_response_and_return!(self, format!("move from {:?} to {:?} can't be perfomed", pos1, pos2), stay);
                    }
                    Some(v) =>{
                        if v.len() == 1{
                           plmove = v[0].clone();
                        }
                        else{
                            loop {
                                self.send_response(vec![format!("Pick a move [0-{}]", v.len()-1).to_string()].into_iter());

                                if let Ok(msg) = self.receiver.recv() {
                                    if let Ok(num) = msg.parse::<u32>() {
                                        if (num as usize) < v.len(){
                                            plmove = v[num as usize].clone();
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let res = self.board.as_mut().unwrap().make_move(pos1, pos2, plmove, &self.sender, &self.receiver);

                match res{
                    Ok(v2) =>{
                        self.match_game_state(v2, stay)
                    }
                    Err(mm) => {
                        send_response_and_return!(self, format!("move from {:?} to {:?} can't be perfomed, {}", pos1, pos2, mm), stay);
                    }
                }
            }
            _ if msg.starts_with("revive ") => {
                let command = msg.trim_start_matches("revive ").trim();
                let args: Vec<&str> = command.split_whitespace().collect();
                if args.len()!=2{
                    send_response_and_return!(self, "Wrong input.\n revieve [cementary index] [position]", stay);
                }

                let (i, p) = (args[0], args[1]);
                let pos = Board::parse_position(&self.board.as_mut().unwrap(), p);
                let idx = i.parse::<u32>();

                if pos.is_none() || idx.is_err(){
                    send_response_and_return!(self, "Wrong input.\n move [start position] [end position]", stay);
                }

                let pos = pos.unwrap(); let idx = idx.unwrap();
                let white = if self.board.as_ref().unwrap().current_player==Player::White {true} else {false};

                let res = self.board.as_mut().unwrap().revive_piece(white,idx , pos, &self.sender, &self.receiver);

                match res{
                    Ok(v2) =>{
                        self.match_game_state(v2, stay)
                    }
                    Err(mm) => {
                        send_response_and_return!(self, format!("revive from {:?} to {:?} can't be perfomed, {}", idx, pos, mm), stay);
                    }
                }
            }
            _ => {
                self.send_response(vec![
                    "Processing input...".to_string(),
                ].into_iter());
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
                self.send_response(vec![
                    "Restarting the game...".to_string(),
                ].into_iter());
                self.board = None;
                Mode::Start
            }
            _ => {
                self.send_response(vec![
                    "Game is over.".to_string(),
                    "Use 'restart' to start again or 'exit' to quit.".to_string(),
                ].into_iter());
                Mode::Finished
            }
        }
    }

    fn match_game_state(&self, state: GameState, stay: Mode) -> Mode{
        match state {
            GameState::BlackWins => {
                send_response_and_return!(self, "Black wins", Mode::Finished);
            }
            GameState::WhiteWins =>{
                send_response_and_return!(self, "White wins", Mode::Finished);
            }
            GameState::Draw => {
                send_response_and_return!(self, "It's a draw", Mode::Finished);
            }
            GameState::Continue => {
                let board_str = format!("{}", self.board.as_ref().unwrap());

                let curr = self.board.as_ref().unwrap().current_player.as_str();
                send_response_and_return!(self, format!("{}'s turn\nstate:\n{}",curr,board_str), stay);
            }
            GameState::Error(em) => {
                send_response_and_return!(self, format!("could not determine result. er: {}", em), stay);
            }

        }
    }
    ///stwórz domyślną pustą planszę
    fn new_default_board(&mut self) {
        self.board = Some(Board::new());
    }

    ///wczytaj planszę z pliku json
    fn load_board(&mut self, json_path: String) -> Result<(), String> {
        if let Ok(board) = Board::from_json({
            match std::fs::read_to_string(&json_path) {
                Ok(s) => s,
                Err(e) => {return Err(format!("{e}"));}
            }
        }) {
            self.board = Some(board);
            return Ok(());
        };
        Err(format!("Could not create boad from: {}", json_path))
    }

    ///wczytaj planszę z jsona
    fn load_board_json(&mut self, json: String) -> Result<(), String> {
        match Board::from_json(json) {
            Ok(board) => {
                self.board = Some(board);
                return Ok(());
            }
            Err(e) => Err(format!("Could not create boad from json: {}", e))
        }
    }

    ///wczytaj figurę z pliku json i dodaj do planszy
    fn load_pieces(&mut self, json_path: String) -> Result<(), String> {
        if let Some(b) = &mut self.board {
            b.add_piece({
                match std::fs::read_to_string(&json_path) {
                    Ok(s) => s,
                    Err(e) => {return Err(format!("{e}"));}
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

    fn create_move(&self) ->Option<PMove>{
        let mut mv = PMove::default_move();
        while let Ok(msg) = self.receiver.recv(){
            match msg.as_str(){
                "show condition" =>{
                    self.send_response(vec![format!("{:?}",mv.condition)].into_iter());
                    continue;
                }
                "show consequences" =>{
                    self.send_response(vec![format!("{:?}",mv.consequences)].into_iter());
                    continue;
                }
                "set condition" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let cnd = self.create_rule();
                    if cnd.is_none(){
                        continue;
                    }
                    mv.condition=Box::new(cnd.unwrap());
                }
                "getstatus" => {
                    self.sender.send(Some("movecreator".to_string())).expect("Failed to send status");
                    continue;
                }
                "add consequence" =>{
                    self.send_response(vec!["[RuleCreator] welcome to RuleCreator".to_string()].into_iter());
                    let csq = self.create_rule();
                    if csq.is_none(){
                        continue;
                    }
                    if mv.consequences.is_none(){
                        mv.consequences=Some(vec![]);
                    }
                    mv.consequences.as_mut().unwrap().push(Box::new(csq.unwrap()));
                }
                "export" =>{
                    self.send_response(vec!["exporting move, going back to create piece".to_string()].into_iter());
                    return Some(mv);
                }
                "cancel" =>{
                    self.send_response(vec!["movecreate cancelled, going back to create piece".to_string()].into_iter());
                    return None;
                }
                _ =>{
                    self.send_response(vec!["unknown command".to_string()].into_iter());
                    continue;
                }
            }
        }
        None
    }

    fn create_rule(&self) ->Option<RuleExpression>{
        let mut stack: Vec<(String, u32)> = vec![];
        let mut cnts = vec![0];
        let mut layers: Vec<RuleExpression> = vec![];
        let l1 = default_calls();
        let l2 = RuleExpression::list();
        while let Ok(msg) = self.receiver.recv() {

            match msg.as_str() {
                "list stack" =>{
                    self.send_response(vec![format!("{:?}", stack)].into_iter());
                    continue;
                }
                "list layers" =>{
                    self.send_response(vec![format!("{:?}", layers)].into_iter());
                    continue;
                }
                "cancel" => {
                    self.send_response(vec!["exiting rule creator".to_string()].into_iter());
                    break;
                },
                "getstatus" => {
                    self.sender.send(Some("rulecreator".to_string())).expect("Failed to send status");
                    continue;
                }
                //finish and export expression, check if all fine
                "export" =>{
                    if layers.len()!=1{
                        self.send_response(vec!["can't create rule".to_string()].into_iter());
                        continue;
                    }
                    self.send_response(vec![format!("successfully exported rule {:?}",layers[0])].into_iter());
                    return Some(layers[0].clone());
                }
                //close current ruleexpr eg: enough commands in and., has to be run with fixed ones like "if" as well
                "close" => {
                    println!("current state:\nstack:{:?}\ncnts:{:?}\nlayers:{:?}",stack,cnts,layers);
                    if stack.len()<1 {
                        self.send_response(vec!["Stack empty, this should not happen".to_string()].into_iter());
                        continue;
                    }
                    let roz = stack.len()-1;
                    let siz = stack[roz].1;
                    if siz!=u32::MAX && siz!= *cnts.last().unwrap(){
                        self.send_response(vec!["wrong number of subexpressions".to_string()].into_iter());
                        continue;
                    }
                    let s2 = layers.len();
                    let tmp:RuleExpression = match stack[roz].0.as_str() {
                        "Diff" =>{
                            RuleExpression::Diff(Box::new(layers[s2-2].clone()), Box::new(layers[s2-1].clone()))
                        }
                        "Pair" =>{
                            RuleExpression::Pair(Box::new(layers[s2-2].clone()), Box::new(layers[s2-1].clone()))
                        }
                        "And" =>{
                            let pom = *cnts.last().unwrap() as usize;
                            RuleExpression::And(layers[(s2-pom)..].to_vec())
                        }
                        "Or" =>{
                            let pom = *cnts.last().unwrap() as usize;
                            RuleExpression::Or(layers[(s2-pom)..].to_vec())
                        }
                        "Not" =>{
                            RuleExpression::Not(Box::new(layers[s2-1].clone()))
                        }
                        "Equals" =>{
                            RuleExpression::Equals(Box::new(layers[s2-2].clone()), Box::new(layers[s2-1].clone()))
                        }
                        "If" =>{
                            RuleExpression::If(Box::new(layers[s2-3].clone()),Box::new(layers[s2-2].clone()), Box::new(layers[s2-1].clone()))
                        }
                        _ =>{
                            self.send_response(vec!["unknown ruleexpression, this should not happen".to_string()].into_iter());
                            return None;
                        }
                    };
                    let x = stack.pop();
                    layers.truncate(s2-(*cnts.last().unwrap() as usize));
                    cnts.pop();
                    layers.push(tmp);
                    self.send_response(vec![format!("successfully closed expr {}",x.unwrap().0)].into_iter());
                    continue;
                }
                "list apicalls" => {
                    self.send_response(l1.iter().map(|x| x.0.clone()));
                    continue;
                },
                "list" =>{
                    self.send_response(l2.iter().map(|x| x.0.clone()));
                    continue;
                },
                "Void" =>{
                    layers.push(RuleExpression::Void);
                    *cnts.last_mut().unwrap() += 1;
                    self.send_response(vec!["successfully created Void".to_string()].into_iter());
                    continue;
                }
                //one line inp
                _ if msg.starts_with("Integer") => {
                    let command = msg.trim_start_matches("Integer").trim();
                    let args: Vec<&str> = command.split_whitespace().collect();

                    if args.len()!=1 {
                        self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                        continue;
                    }
                    let x = args[0].parse::<i32>();
                    if x.is_ok(){
                        layers.push(RuleExpression::Integer(x.unwrap()));
                        *cnts.last_mut().unwrap() += 1;
                        self.send_response(vec!["successfully created Integer".to_string()].into_iter());
                    }
                    else{
                        self.send_response(vec!["Not an integer".to_string()].into_iter());
                    }
                    continue;
                }
                _ if msg.starts_with("Boolean") => {
                    let command = msg.trim_start_matches("Boolean").trim();
                    let args: Vec<&str> = command.split_whitespace().collect();

                    if args.len()!=1 {
                        self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                        continue;
                    }
                    let x = args[0].parse::<bool>();
                    if x.is_ok(){
                        layers.push(RuleExpression::Boolean(x.unwrap()));
                        *cnts.last_mut().unwrap() += 1;
                        self.send_response(vec!["successfully created Boolean".to_string()].into_iter());
                    }
                    else{
                        self.send_response(vec!["Not a bool".to_string()].into_iter());
                    }
                    continue;
                }
                _ if msg.starts_with("Variable") => {
                    let command = msg.trim_start_matches("Variable").trim();
                    let args: Vec<&str> = command.split_whitespace().collect();
                    println!("erasdsad: {:?}", args);
                    if args.len()!=1 {
                        self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                        continue;
                    }

                    layers.push(RuleExpression::Variable(args[0].to_string()));
                    *cnts.last_mut().unwrap() += 1;
                    self.send_response(vec!["successfully created Variable".to_string()].into_iter());
                    continue;
                }
                _ if msg.starts_with("ApiCall") => {
                    let command = msg.trim_start_matches("ApiCall").trim();
                    let args: Vec<&str> = command.split_whitespace().collect();

                    if args.len()<1 {
                        self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                        continue;
                    }

                    let call = args[0];
                    if l1.contains_key(call){
                        let temp = &l1[call];
                        match temp.0{
                            //unlimited args
                            u32::MAX =>{

                            },
                            4294967294 =>{
                                //not 2 3 bc call name
                                if args.len()!=3 && args.len()!=4{
                                    self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                                    continue;
                                }
                            }
                            _ =>{
                                if args.len()-1 != temp.0 as usize{
                                    self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                                    continue;
                                }
                            }

                        }
                    }
                    else{
                        self.send_response(vec!["No such call".to_string()].into_iter());
                        continue;
                    }

                    layers.push(RuleExpression::ApiCall(call.to_string(), args.into_iter().skip(1).map(|x| x.to_string()).collect()));
                    *cnts.last_mut().unwrap() += 1;
                    self.send_response(vec!["successfully created ApiCall".to_string()].into_iter());
                    continue;
                }
                _ if msg.starts_with("Error") => {
                    let command = msg.trim_start_matches("Error").trim();
                    let args: Vec<&str> = command.split_whitespace().collect();

                    if args.len()!=1 {
                        self.send_response(vec!["Wrong number of arguments".to_string()].into_iter());
                        continue;
                    }

                    layers.push(RuleExpression::Err(args[0].to_string()));
                    *cnts.last_mut().unwrap() += 1;
                    self.send_response(vec!["successfully closed Error".to_string()].into_iter());
                    continue;
                }
                //ruleexpr that expands the stack
                _ if l2.contains_key(&msg) =>{
                    stack.push((msg.clone(), l2[&msg].0));
                    *cnts.last_mut().unwrap() += 1;
                    cnts.push(0);
                    self.send_response(vec![format!("successfully opened {}",msg)].into_iter());
                    continue;
                }
                _ =>{
                    self.send_response(vec!["unknown command".to_string()].into_iter());
                }
            }
        }
        None
    }

}
