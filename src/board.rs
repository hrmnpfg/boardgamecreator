use std::sync::mpsc::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::player::Player;
use crate::GameState;
use crate::RuleExpression;
use crate::PMove;
use crate::piece::Piece;
use std::error::Error;
use std::collections::HashMap;
use std::collections::VecDeque;


macro_rules! evaluate_rules {
    ($board:ident, $pos:expr, $field:ident, $context:expr, $depth:expr, $send:expr, $receive:expr) => {
        if let Some(piece) = &$board.board[$pos.0 as usize][$pos.1 as usize] {
            if let Some(rule) = piece.$field.clone() {
                rule.iter().for_each(|x| { let _ = x.evaluate($board, &$context, $depth, $send, $receive);   });
            }
        }
    }
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
pub struct Board
{
    pub size: (u32,u32),
    pub board: Vec<Vec<Option<Piece>>>,
    pub pieces: HashMap<String, String>, // all figure types in current game
    pub cementaries: (Vec<Piece>,Vec<Piece>),
    pub global_rules: Vec<()>,
    pub endcondition: Option<RuleExpression>, // if none then end if one of them wins
    pub wincondition: (RuleExpression, RuleExpression), //if none or both evaluates to true its a draw
    pub cards: (Vec<u32>, Vec<u32>),
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
pub fn default_calls() ->HashMap<String, (u32,String)>{
    HashMap::from([("is_empty".to_string(),(1, "checks if given named position is empty\neg: is_empty new_position".to_string())), ("is_pempty".to_string(),(1, "checks if given position is empty\neg: is_pempty 1,1".to_string())), ("is_pattacked".to_string(),(1,"checks if piece at given position is attacked\neg: is_pattacked 1,1".to_string())),
             ("is_posattacked".to_string(),(1,"checks if given position is attacked\neg: is_posattacked 1,1".to_string())), ("is_ptype".to_string(),(2, "checks if piece at a position is of given type\neg:is_ptype 1,1 pawn".to_string())), ("is_opponent".to_string(),(1, "checks if given named postion has opponent piece\neg: is_opponent new_position".to_string())),
             ("is_ally".to_string(),(1, "checks if given named postion has ally piece\neg: is_ally new_position".to_string())), ("is_different".to_string(),(2, "checks if two named positions are different\neg: is_different new_position old_position".to_string())), ("in_bounds".to_string(),(1,"checks if given position is in bounds\neg: in_bounds 1,1".to_string())),
             ("is_path_blocked".to_string(),(0, "checks if path between old_position and new_position is blocked\neg: is_path_blocked".to_string())), ("get_x".to_string(),(0, "returns vertical coordinate of old_position\neg: get_x".to_string())), ("get_y".to_string(),(0,"returns horizontal coordinate of old_position\neg: get_y".to_string())),
             ("get_target_x".to_string(),(0,"returns vertical coordinate of new_position\neg: get_target_x".to_string())), ("get_target_y".to_string(),(0,"returns horizontal coordinate of new_position\neg: get_target_y".to_string())), ("get_fvar".to_string(),(2, "returns value of piece at a given named position\neg: get_fvar old_position move_count".to_string())),
             ("get_pvar".to_string(), (2, "returns value of a piece at a given position\neg: get_pvar 1,1 move_count".to_string())), ("get_idvar".to_string(),(2, "returns value of piece variable of piece with given id\neg: get_idvar 1 move_count".to_string())), ("increase_var".to_string(),(3,"increase given variable of a piece at given named position\neg: increase_var new_position move_count 13".to_string())),
             ("change_type".to_string(), (u32::MAX, "change type of piece at new_position to one of listed ones\neg: change_type pawn rook".to_string())), ("forward".to_string(), (u32::MAX-1, "returns vertical difference to position\nforward [[position|variable] value ]| [history src|dest index]\neg: forward variable new_postion".to_string())), ("history".to_string(), (u32::MAX-1, "returns move from history\nhistory [src|dest] idx (x|y)\neg: history dest 0 x".to_string())), ("kill".to_string(),(u32::MAX-1 ,"kills piece at specified position\nkill [[position|variable] value ]| [history src|dest index]\neg: kill history dest 0".to_string())), ("piece_on_board".to_string(), (1, "checks if there is at least one piece of given type on board\neg: piece_on_board pawn".to_string())), ("piece_on_board_cnt".to_string(),(1,"returns number of pieces of given type on board\neg: piece_on_board_cnt pawn".to_string())), ("player_piece_on_board".to_string(), (2, "checks if given player has at least one piece of given type on board\neg: player_piece_on_board  White pawn".to_string())), ("player_piece_on_board_cnt".to_string(), (2, "returns number of pieces of given type given player has on board\neg: player_piece_on_board_cnt Black pawn".to_string())),("move_piece".to_string(),(2, "moves piece at src to dst\nmove_piece src dst\neg: move_piece 1,1 2,1".to_string())), ("is_same_line".to_string(), (3, "checks if all position on give line are the same\nis_same_line [position1] [postion2] [piece|owner]".to_string()))
                  ])
}

impl Board
{
    ///tworzy planszę na podstawie pliku json
    pub fn from_json(s: String) -> Result<Self, Box<dyn Error>>
    {
        let mut board: Board =  serde_json::from_str(&s)?;

        let parsed: Value =  serde_json::from_str(&s)?;

        let pain3 = serde_json::from_value(parsed["initialboard"].clone());

        if pain3.is_err(){
            println!("does not have initialboard");
            return Ok(board);
        }
        let pain2: Vec<Vec<Option<(String, String, i32)>>> = pain3.unwrap();
        /*
         if we want to have id's
             we can have them predefined in json then we have to change initialboard to just hold options of piece instead of pair figure name and owner. or hold triples of figure piece and id'

             or we can do that on init, then better not to touch the json.

             for now i go with triples
         */
        board.board = pain2.iter().map(|x| x.iter().map(|y|
            if let Some((p1,p2,p3)) = y {
                if !board.pieces.contains_key(p1) {
                    return None; //incorrect json
                }
                let mut piece = Piece::new(board.pieces[p1].clone()).ok().unwrap(); //assume jsons for figures are ok for now

                piece.set_owner(
                    unwrap_or_return!(
                        Player::from_str(p2),
                        None //incorrect json
                    )
                );

                match &mut piece.memory {
                    Some(hashmap) => {
                        hashmap.insert("id".to_string(), RuleExpression::Integer(*p3));
                    }
                    None =>  {piece.memory = Some({
                                let mut map = HashMap::new();
                                map.insert("id".to_string(), RuleExpression::Integer(*p3));
                                map
                            });
                    }
                }

                Some(piece)
            }
            else
            {
                None
            }
        ).collect()).collect();

        Ok(board)
    }

    ///tworzy domyślną, pustą planszę
    pub fn new() -> Board
    {
        let mut board = Vec::with_capacity(8);
        for _i in 0..8
        {
            let mut row = Vec::with_capacity(8);
            for _j in 0..8
            {
                row.push(None);
            }
            board.push(row);
        }
        Board
        {
            size: (8,8),
            board,
            pieces: HashMap::new(),
            cementaries: (Vec::new(), Vec::new()),
            global_rules: Vec::new(), //no way in hell im making this longer with try new
            endcondition:  Some(RuleExpression::Boolean(false)),
            wincondition: (RuleExpression::Boolean(false), RuleExpression::Boolean(false)),
            history: VecDeque::new(),
            history_size: 1,
            cards: (Vec::new(), Vec::new()),
            current_player: Player::White,
            revive: RuleExpression::Boolean(false),
        }
    }

    pub fn add_piece(&mut self, json: String) -> Result<(), String> {
        match serde_json::from_str::<Value>(&json)
        {
            Ok(data) => {
                println!("dataaa: {:?}",data);
                if let Some(id) = data["id"].as_str()
                {
                    //TODO add handling trying to add pieces with the same id!!!!
                    self.pieces.insert(id.to_string(), json);
                    Ok(())
                }
                else {
                    Err("this shouldn't be possible".to_string())
                }
            }
            Err(e) => Err(e.to_string())
        }

    }

    pub fn add_piece2(&mut self, piece: Piece) -> Result<(), String> {
        if self.pieces.contains_key(&piece.id){
            Err("piece already on board".to_string())
        }
        else{
            let pom = serde_json::to_string(&piece);
            if pom.is_err(){
                Err("something when wrong with to_sting on this piece".to_string())
            }
            else{
                self.pieces.insert(piece.id, pom.unwrap());
                Ok(())
            }
        }
    }

    ///dodaj ruch do historii
    pub fn add_history(&mut self, pmove: PMove, from: (u32, u32), to: (u32, u32)){
        if self.history_size == 0 {
            return ();
        }

        if self.history.len() == self.history_size as usize{
            self.history.pop_front();
        }

        self.history.push_back((pmove, (from, to)))
    }

    pub fn get_history(&mut self, idx: u32) -> Option<(PMove,((u32, u32),(u32, u32)))>{
        if idx>=self.history_size || (idx as usize)>= self.history.len(){
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

        let mut x = fx as i32;
        let mut y = fy as i32;

        loop {
            x = x+dx;
            y = y+dy;

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
    pub fn parse_position(&self, pos_str: &str) -> Option<(u32, u32)> { //"1, 2"
        let parts: Vec<&str> = pos_str.split(',').collect();
        if parts.len() == 2 {
            let x = parts[0].parse().unwrap_or(0);
            let y = parts[1].parse().unwrap_or(0);
            Some((x, y))
        } else {
            None
        }
    }

    ///parsuje pozycję zapisaną jako string na (u32, u32)
    pub fn parse_position_relative(&self, pos_str: &str) -> Option<(i32, i32)> {
        let parts: Vec<&str> = pos_str.split(',').collect();
        if parts.len() == 2 {
            let x = parts[0].parse().unwrap_or(0);
            let y = parts[1].parse().unwrap_or(0);
            Some((x, y))
        } else {
            None
        }
    }

    ///lista wszystkich pozycji osiągalnych dla figury z pozycji
    pub fn get_possible_positions(&mut self, old_position: (u32, u32), depth: u32) -> Option<Vec<(u32, u32)>> {
        let (x, y) = old_position;
        if x >= self.size.0 || y >= self.size.1
        {
            return None;
        }
        //                            V perfectly safe, as checked above, just don't want to deal with error handling
        let piece: &Piece = match unsafe {
            self.board.get_unchecked(old_position.0 as usize).get_unchecked(old_position.1 as usize).as_ref()
        }
        {
            Some(f) => f,
            None => return None
        };
        let mut ret = Vec::new();
        let possible_moves = piece.possiblemoves.clone();
        let movecondition = piece.movecondition.clone();
        let mut context = HashMap::new(); // TODO maybe adding context as argument, for now not needed
        context.insert(String::from("old_position"),format!("{},{}",x,y).to_string());

        for x in 0..self.size.0 {
            for y in 0..self.size.1 {
                context.insert(String::from("new_position"),format!("{},{}",x,y).to_string());
                // TODO later it should return pairs of positions and pmove.

                let move_possible =  possible_moves.iter().any(|pmove| {
                    matches!(pmove.condition.evaluate(self, &context, depth, None, None), RuleExpression::Boolean(true))
                });

                let condition_met = movecondition.as_ref().map_or(true, |condition| {
                    matches!(condition.evaluate(self, &context, depth, None, None), RuleExpression::Boolean(true))
                });

                if move_possible && condition_met {
                    ret.push((x, y));
                }
            }
        }

        Some(ret)
    }

    ///lista wszystkich ruchów figury z pozycji
    pub fn get_possible_moves(&mut self, old_position: (u32, u32), depth: u32) -> Option<Vec<((u32, u32), PMove)>> {
        let (x, y) = old_position;
        if x >= self.size.0 || y >= self.size.1
        {
            return None;
        }
        //                            V perfectly safe, as checked above, just don't want to deal with error handling
        let piece: &Piece = match unsafe {
            self.board.get_unchecked(old_position.0 as usize).get_unchecked(old_position.1 as usize).as_ref()
        }
        {
            Some(f) => f,
            None => return None
        };
        let mut ret = Vec::new();
        let possible_moves = piece.possiblemoves.clone();
        let movecondition = piece.movecondition.clone();
        let mut context = HashMap::new();// TODO maybe adding context as argument, for now not needed

        if movecondition.as_ref().is_some_and(|v| v.evaluate(self, &context, depth, None, None)==RuleExpression::Boolean(false) ){
            return None;
        }

        context.insert(String::from("old_position"),format!("{},{}",x,y).to_string());

        for x in 0..self.size.0 {
            for y in 0..self.size.1 {
                context.insert(String::from("new_position"),format!("{},{}",x,y).to_string());

                let move_possible =  possible_moves.iter().filter(|pmove| {
                    matches!(pmove.condition.evaluate(self, &context, depth, None, None), RuleExpression::Boolean(true))
                }).map(|v| ((x,y),v.clone()));

                ret.extend(move_possible);
            }
        }

        if ret.is_empty(){
            return None;
        }
        Some(ret)
    }

    ///lista ruchów z a do b
    pub fn get_moves_to(&mut self, old_position: (u32, u32), new_position: (u32, u32), depth: u32) -> Option<Vec<PMove>> {
        let (x, y) = old_position;
        let (x2, y2) = new_position;
        if x >= self.size.0 || y >= self.size.1 || x2>= self.size.0 || y2 >= self.size.1
        {
            return None;
        }
        //                            V perfectly safe, as checked above, just don't want to deal with error handling
        let piece: &Piece = match unsafe {
            self.board.get_unchecked(old_position.0 as usize).get_unchecked(old_position.1 as usize).as_ref()
        }
        {
            Some(f) => f,
            None => return None
        };
        let mut ret = Vec::new();
        let possible_moves = piece.possiblemoves.clone();
        let movecondition = piece.movecondition.clone();
        let mut context = HashMap::new();// TODO maybe adding context as argument, for now not needed


        context.insert(String::from("old_position"),format!("{},{}",x,y).to_string());
        context.insert(String::from("new_position"),format!("{},{}",x2,y2).to_string());

        if movecondition.as_ref().is_some_and(|v| v.evaluate(self, &context, depth, None, None)==RuleExpression::Boolean(false) ){
            return None;
        }

        let move_possible =  possible_moves.iter().filter(|pmove| {
            matches!(pmove.condition.evaluate(self, &context, depth, None, None), RuleExpression::Boolean(true))
        }).map(|v| v.clone());

        ret.extend(move_possible);

        if ret.is_empty(){
            return None;
        }

        Some(ret)
    }

    pub fn revive_piece(&mut self, white: bool, idx: u32, position: (u32, u32), sender: &Sender<Option<String>>, receiver: &Receiver<String> ) -> Result<GameState,String>{

        let mut context = HashMap::new();
        context.insert(String::from("new_position"),format!("{},{}",position.0, position.1).to_string());
        if self.revive.clone().evaluate(self, &context , 0, Some(sender), Some(receiver)) == RuleExpression::Boolean(false){
            return Err("Cannot revive".to_string());
        }

        let piece;
        if white {
            if self.cementaries.0.len() <= idx as usize{
                return Err("no piece at that cementary index".to_string());
            }
            piece = self.cementaries.0[idx as usize].clone();
            self.cementaries.0.swap_remove(idx as usize);
        }
        else{
            if self.cementaries.1.len() <= idx as usize{
                return Err("no piece at that cementary index".to_string());
            }
            piece = self.cementaries.1[idx as usize].clone();
            self.cementaries.1.swap_remove(idx as usize);
        }

        let mut kill = false;
        if let Some(ref piece2) = self.board[position.0 as usize][position.1 as usize] {
            //TODO add if piece can be killed, for example if i can kill my own field
            match piece2.owner {
                Player::White => self.cementaries.0.push(piece2.clone()), // white pieces killed, cemetery
                Player::Black => self.cementaries.1.push(piece2.clone()), // Black pieces killed, cemetery
            }
            kill = true;
        }
        else {
            // leave it for now since we might want to do actions only if not kill
        }
        if kill { evaluate_rules!(self, position, deathrattle, context, 0, Some(sender), Some(receiver)); }

        self.board[position.0 as usize][position.1 as usize] = Some(piece);
        return self.get_game_state(&context);
    }
    ///returns the list of not current_player positions that attack a given position,
    pub fn get_attackers_player(&mut self, position: (u32, u32), pp: Player, depth: u32 ) -> Option<Vec<(u32, u32)>> {
        //assumes to get a valid position

        let mut attackers: Vec<(u32, u32)> = vec![];
        for x in 0..self.size.0{
            for y in 0..self.size.1{
                if (x,y) != position && self.board[x as usize][y as usize].as_ref().is_some_and(|v| v.owner != pp) && self.get_moves_to((x,y), position, depth).is_some(){
                    attackers.push((x,y));
                }
            }
        }
        if attackers.len()!=0{
            return Some(attackers);
        }
        None
    }

    ///returns true if position is attacked, just to avoid always checking if is some
    pub fn is_attacked_player(&mut self, position: (u32,u32), opponent: bool, depth: u32) -> bool {
        let pp = if opponent {self.current_player} else{ !self.current_player };
        let old = self.current_player;
        self.current_player = !pp;
        let a = self.get_attackers_player(position, pp, depth).is_some();
        self.current_player = old;
        a
    }

    ///returns true if piece at position is attacked, just to avoid always checking if is some
    pub fn is_attacked_piece(&mut self, position: (u32,u32), opponent:bool, depth:u32) -> bool {
        if self.board[position.0 as usize][position.1 as usize].is_none(){
            return false;
        }
        let old = self.current_player;
        let mut pp = unwrap_or_return!(self.board[position.0 as usize][position.1 as usize].clone(), false).owner;
        if !opponent {
            pp = !pp;
        }
        self.current_player = !pp;
        let a = self.get_attackers_player(position, pp, depth).is_some();
        self.current_player = old;
        a
    }

    ///returns the list of position that can attack given position
    pub fn get_attackers(&mut self, position: (u32, u32), depth:u32) -> Option<Vec<(u32, u32)>> {
        //assumes to get a valid position
        let mut attackers: Vec<(u32, u32)> = vec![];
        for x in 0..self.size.0{
            for y in 0..self.size.1{
                if (x,y) != position && self.get_moves_to((x,y), position, depth).is_some(){
                    attackers.push((x,y));
                }
            }
        }
        if attackers.len()!=0{
            return Some(attackers);
        }
        None
    }

    ///returns true if position is attacked, just to avoid always checking if is some
    pub fn is_attacked(&mut self, position: (u32,u32), depth:u32) -> bool {
        self.get_attackers(position, depth).is_some()
    }

    ///returns positions that piece at a given position attacks
    pub fn get_attacked(&mut self, position: (u32, u32), depth: u32) -> Option<Vec<(u32, u32)>>{
        // for now i assume that pieces know whether they can kill ally pieces
        Some(unwrap_or_return!(self.get_possible_positions(position, depth), None).into_iter().filter(|(x,y)| self.board[*x as usize][*y as usize].is_some()).collect())
    }

    ///returns true if piece at given position can attack
    pub fn can_attack(&mut self, position: (u32,u32), depth: u32) -> bool {
        self.get_attacked(position, depth).is_some()
    }

    ///returns positions that piece at a given position attacks
    pub fn get_attacked_player(&mut self, position: (u32, u32), opponent: bool, depth: u32) -> Option<Vec<(u32, u32)>>{
        if self.board[position.0 as usize][position.1 as usize].is_none(){
            return None;
        }

        let pp : Player = if opponent {!(self.board[position.0 as usize][position.1 as usize].as_ref().unwrap().owner)} else {self.board[position.0 as usize][position.1 as usize].as_ref().unwrap().owner};

        Some(unwrap_or_return!(self.get_possible_positions(position, depth), None).into_iter().filter(|(x,y)| self.board[*x as usize][*y as usize].as_ref().is_some_and(|v| v.owner!= pp)).collect())
    }

    ///returns true if piece at given position can attack
    pub fn can_attack_player(&mut self, position: (u32,u32), opponent: bool, depth: u32) -> bool {
        self.get_attacked_player(position, opponent, depth).is_some()
    }

    ///returns value of variable for piece at given position
    pub fn get_piece_var(&mut self, position: (u32, u32), variable: &str) ->RuleExpression {
        if let Some(ref mut piece) = self.board[position.0 as usize][position.1 as usize] {
                    match &mut piece.memory {
                        Some(hmm) => {
                            match hmm.get(variable) {
                                Some(val) => {
                                    return val.clone();
                                },
                                None => {
                                    return RuleExpression::Err(format!("piece {} does not have {} variable in {}", piece.name, variable, "get_piece_var func"));
                                }
                            }
                        }
                        None => {
                           return RuleExpression::Err(format!("piece {} does not have memory in {}", piece.name, "get_piece_var func"));
                        }
                    }
                }
                else {
                    return RuleExpression::Err(format!("no piece at given position in {}", "get_piece_var func"));
                }
    }

    pub fn match_and_get_position(&mut self, args: Vec<&str>, context: &HashMap<String,String>) -> Option<(u32, u32)>{
        match args [0] {
            "position" => {
                self.get_position(args[1])
            }
            "variable" => {
                if context.contains_key(args[1]){
                    self.get_position(&context[args[1]])
                }
                else {
                    None
                }
            }
            // dla history jest history src|dest number
            "history" => {
                let pomi = unwrap_or_return!(self.get_history(unwrapres_or_return!(args[2].parse::<u32>(), None )), None);

                match args[1] {
                    "src" | "source" => {
                        Some(pomi.1.0)
                    }
                    "dst" | "dest" | "destination" =>{
                        Some(pomi.1.1)
                    }
                    _ => {
                        return None;
                    }
                }
            }

            _ => {
                None
            }
        }

    }
    ///uproszczenie zapytania api do wartości
    pub fn call_api(&mut self, api: &str, args: Vec<&str>, context: &HashMap<String,String>, depth: u32, sender: Option<&Sender<Option<String>>>, receiver: Option<&Receiver<String>>) -> RuleExpression {
        match api {
            "is_empty" =>  {
                RuleExpression::Boolean(!self.has_piece_at(
                    unwrap_or_return!(
                        self.get_position(unwrap_or_return!(
                            context.get(args[0]),
                            RuleExpression::Err(format!("no {} in {}",args[0],api))
                        )),
                        RuleExpression::Err(String::from("is_empty"))
                    )
                ))
            },

            "is_pempty" =>  {
                RuleExpression::Boolean(!self.has_piece_at(
                    unwrap_or_return!(
                        self.get_position(args[0]),
                        RuleExpression::Err(String::from("is_pempty"))
                    )
                ))
            },

            "is_pattacked" => {
                // TODO add changes when you can kill your own pieces

                let pos = unwrap_or_return!(
                        self.get_position(args[0]),
                        RuleExpression::Err(String::from("is_pattacked"))
                    );
                let x = RuleExpression::Boolean(self.is_attacked_piece(pos, true, depth));
                x
            },

            "is_posattacked" => {
                // TODO add changes when you can kill your own pieces

                let pos = unwrap_or_return!(
                        self.get_position(args[0]),
                        RuleExpression::Err(String::from("is_posattacked"))
                    );
                let x = RuleExpression::Boolean(self.is_attacked_player(pos, true, depth));
                x
            },

            "is_ptype" => {
                if args.len()!=2 {
                    return RuleExpression::Err("wrong number of arguments in is_ptype".to_string());
                }
                let pos = unwrap_or_return!(
                        self.get_position(args[0]),
                        RuleExpression::Err(String::from("is_pattacked"))
                    );
                RuleExpression::Boolean(unwrap_or_return!(self.board[pos.0 as usize][pos.1 as usize].clone(), RuleExpression::Err("no piece at given position".to_string())).id == args[1])
            }

            "is_opponent" => {
                let pos = unwrap_or_return!(self.get_position(unwrap_or_return!(
                            context.get(args[0]),
                            RuleExpression::Err(format!("no {} in {}",args[0],api))
                        )), RuleExpression::Err(format!("{} is not a position {}",args[0],api)));

                if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {
                    return RuleExpression::Boolean(piece.owner!=self.current_player);
                }

                RuleExpression::Boolean(false)
            }

            "is_ally" => {
                let pos = unwrap_or_return!(self.get_position(unwrap_or_return!(
                            context.get(args[0]),
                            RuleExpression::Err(format!("no {} in {}",args[0],api))
                        )), RuleExpression::Err(format!("{} is not a position {}",args[0],api)));

                if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {
                    return RuleExpression::Boolean(piece.owner==self.current_player);
                }

                RuleExpression::Boolean(false)
            }

            "is_player" => {
                if args.len()!=2{
                    return RuleExpression::Err("wrong number of args".to_string());
                }
                let pos = unwrap_or_return!(
                        self.get_position(args[0]),
                        RuleExpression::Err(String::from("is_pempty"))
                    );

                if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {
                    return match args[1] {
                        "White" => RuleExpression::Boolean(piece.owner==Player::White),
                        "Black" => RuleExpression::Boolean(piece.owner==Player::Black),
                        _ => RuleExpression::Boolean(false),
                    }
                }

                RuleExpression::Boolean(false)
            }

            "is_different" => RuleExpression::Boolean(
                self.get_position(
                    unwrap_or_return!(
                        context.get(args[0]),
                        RuleExpression::Err(format!("no {} in {}",args[0],api))
                    )
                )
                !=
                self.get_position(
                    unwrap_or_return!(
                        context.get(args[1]),
                        RuleExpression::Err(format!("no {} in {}",args[1],api))
                    )
                )
            ),

            "in_bounds" => RuleExpression::Boolean(self.is_position_valid(args[0])),

            "is_path_blocked" => RuleExpression::Boolean(
                self.is_path_blocked(
                    unwrap_or_return!(
                        self.get_position(unwrap_or_return!(
                            context.get("old_position"),
                            RuleExpression::Err(format!("no old_position in {}",api))
                        )),
                        RuleExpression::Err(format!("old_position in {}",api))
                    ),
                    unwrap_or_return!(
                        self.get_position(unwrap_or_return!(
                            context.get("new_position"),
                            RuleExpression::Err(format!("no new_position in {}",api))
                        )),
                        RuleExpression::Err(format!("new_position in {}",api))
                    )
                )
            ),

            "get_x" => RuleExpression::Integer(
                unwrap_or_return!(
                    self.get_position(unwrap_or_return!(
                        context.get("old_position"),
                        RuleExpression::Err(format!("no old_position in {}",api))
                    )),
                    RuleExpression::Err(format!("old_position in {}",api))
                ).0 as i32),

            "get_y" =>
                RuleExpression::Integer(
                unwrap_or_return!(
                    self.get_position(unwrap_or_return!(
                        context.get("old_position"),
                        RuleExpression::Err(format!("no old_position in {}",api))
                    )),
                    RuleExpression::Err(format!("old_position in {}",api))
                ).1 as i32),

            "get_target_x" => {
                let pos = unwrap_or_return!(
                    self.get_position(unwrap_or_return!(
                        context.get("new_position"),
                        RuleExpression::Err(format!("no new_position in {}",api))
                    )),
                    RuleExpression::Err(format!("new_position in {}",api))
                );
                RuleExpression::Integer(pos.0 as i32)
            },

            "get_target_y" => {
                let pos = unwrap_or_return!(
                    self.get_position(unwrap_or_return!(
                        context.get("new_position"),
                        RuleExpression::Err(format!("no new_position in {}",api))
                    )),
                    RuleExpression::Err(format!("new_position in {}",api))
                );
                RuleExpression::Integer(pos.1 as i32)
            },
            //get variable from piece at position
            "get_fvar" => {
                if args.len()!=2{
                    return RuleExpression::Err(format!("wrong number of arguments in {}", api));
                }
                let c = unwrap_or_return!(
                    context.get(args[0]),
                    RuleExpression::Err(format!("no {} in {}", args[0], api))
                );
                let pos = unwrap_or_return!(
                    self.get_position(c),
                    RuleExpression::Err(format!("get_position {} in {}", c, api))
                );

                self.get_piece_var(pos,args[1])
            }

            "get_pvar" => {
                if args.len()!=2{
                    return RuleExpression::Err(format!("wrong number of arguments in {}", api));
                }

                let pos = unwrap_or_return!(
                    self.get_position(args[0]),
                    RuleExpression::Err(format!("get_position {} in {}", args[0], api))
                );

                self.get_piece_var(pos,args[1])
            }

            //get var by id ONLY WORKS FOR PIECES ON BOARD!!!!!
            "get_idvar" => {
                //TODO add similar thing but with pieces on cementary maybe????
                if args.len() != 2 {
                    return RuleExpression::Err(format!("wrong number of arguments in {}", api));
                }
                let id = RuleExpression::Integer(unwrapres_or_return!(args[0].parse::<i32>(), RuleExpression::Err(format!("Not a valid id in {}", api))));
                for x in 0..self.size.0 {
                    for y in 0..self.size.1 {
                        if self.get_piece_var((x,y), "id") == id {
                            return self.get_piece_var((x,y), args[1]);
                        }
                    }
                }
                return RuleExpression::Err(format!("No piece with id {} on ",api))
            }

            "increase_var" => {
                let c = unwrap_or_return!(
                    context.get(args[0]),
                    RuleExpression::Err(format!("no {} in {}", args[0], api))
                );
                let pos = unwrap_or_return!(
                    self.get_position(c),
                    RuleExpression::Err(format!("get_position {} in {}", c, api))
                );
                let num = args[2].parse::<i32>().unwrap_or(0);
                if let Some(ref mut piece) = self.board[pos.0 as usize][pos.1 as usize] {
                    match &mut piece.memory {
                        Some(hashmap) => {
                            match hashmap.get(args[1]) {
                                Some(val) => {
                                    if let RuleExpression::Integer(num2) = val
                                    {
                                        hashmap.insert(args[1].to_string(), RuleExpression::Integer(*num2 + num));
                                    }
                                },
                                None => {
                                    hashmap.insert(args[1].to_string(), RuleExpression::Integer(num));
                                }
                            }
                        }
                        None => {
                            piece.memory = Some({
                                let mut map = HashMap::new();
                                map.insert(args[1].to_string(), RuleExpression::Integer(num));
                                map
                            })
                        }
                    }
                }
                RuleExpression::Void
            },

            "change_type" => {
                if receiver.is_none() || sender.is_none(){
                    return RuleExpression::Err(format!("Cannot evaluate without receiver or sender in {}",api));
                }
                self.send_response(vec![format!("change to one of available types: {:?}", args)].into_iter(),sender.unwrap());


                let ret = receiver.unwrap().recv();
                match ret {
                    Ok(input) =>{
                        let input = input.trim();
                        if self.pieces.contains_key(input) && args.iter().any(|x| x.to_string() == input) {
                            let mut new_piece = Piece::new(self.pieces[input].clone()).expect("a");
                            let old_position =
                            unwrap_or_return!(
                                self.get_position(unwrap_or_return!(
                                    context.get("new_position"),
                                    RuleExpression::Err(format!("no new_position in {}",api))
                                )),
                                RuleExpression::Err(format!("new_position in {}",api))
                            );
                            if let Some(old_piece) = self.board[old_position.0 as usize][old_position.1 as usize].take() {
                                new_piece.memory = old_piece.memory;
                                new_piece.owner = old_piece.owner;
                                self.board[old_position.0 as usize][old_position.1 as usize] = Some(new_piece);
                            }
                            else {
                                return RuleExpression::Err(format!("No piece at {:?} in {}",old_position, api));
                            }
                        }
                        else{
                            self.send_response(vec![format!("can't change to: {:?}", input)].into_iter(),sender.unwrap());
                            //return self.call_api("change_type", args, context); //rekurencja?
                            return RuleExpression::Err(format!("can't change to {} in {}", input, api));
                        }
                        RuleExpression::Void
                    }
                    Err(err) =>{
                        println!("sth went wrong with inp");
                        RuleExpression::Err(err.to_string())
                    }
                }

            },

            "forward" => {
                // forward [[position|variable] value ]| [history src|dest index]

                if !((args.len()==2 && args[0]!="history") || (args.len()==3 && args[0]=="history")) {
                    return RuleExpression::Err(format!("wrong number of arguments in {} args:{}",api, args.len()));
                }

                let pos = unwrap_or_return!(
                    self.get_position(unwrap_or_return!(
                        context.get("old_position"),
                        RuleExpression::Err(format!("no old_position in {}",api))
                    )),
                    RuleExpression::Err(format!("old_position in {}",api))
                );

                let pos2 = unwrap_or_return!(self.match_and_get_position(args, context), RuleExpression::Err(format!("could not get position in {}", api))); //possible worse bc now not a specific error but more managable

                if let Some(ref piece) = self.board[pos.0 as usize][pos.1 as usize] {

                    match piece.owner {
                        Player::White => return RuleExpression::Integer((pos.0 as i32)-(pos2.0 as i32)),
                        Player::Black => return RuleExpression::Integer((pos2.0 as i32)-(pos.0 as i32))
                    }
                }
                else {
                    return RuleExpression::Err(format!("No piece at {:?} position in {}",pos, api));
                }

            },

            "history" => {
                // history src|dest inx (x|y)
                // TODO later(or when implementing a game that needs it) modify or add other commad for also taking a rule
                if args.len()<2 || args.len()>3{
                    return RuleExpression::Err(format!("wrong amount of argument in {}", api));
                }

                let pom = unwrap_or_return!(self.get_history(unwrapres_or_return!(args[1].parse::<u32>(), RuleExpression::Err(format!("wrong index in {}", api)))),RuleExpression::Err(format!("error with taking entry from history in {}", api)));

                let pom1 = match args[0] {
                    "src" | "source" => {
                        pom.1.0
                    }
                    "dst" | "dest" | "destination" =>{
                        pom.1.1
                    }
                    _ => {
                        return RuleExpression::Err(format!("wrong first argument in {}", api));
                    }
                };

                if args.len()==2{
                    return RuleExpression::Pair(Box::new(RuleExpression::Integer(pom1.0 as i32)), Box::new(RuleExpression::Integer(pom1.1 as i32)));
                }

                match args[2] {
                    "x" => {
                        RuleExpression::Integer(pom1.0 as i32)
                    }
                    "y" =>{
                        RuleExpression::Integer(pom1.1 as i32)
                    }
                    _ => {
                        RuleExpression::Err(format!("wrong third argument in {}", api))
                    }
                }
            },

            //kills piece at specified position
            "kill" => {
                // kill [[position|variable] value ]| [history src|dest index]
                if receiver.is_none() || sender.is_none(){
                    return RuleExpression::Err(format!("Cannot evaluate without receiver or sender in {}",api));
                }

                if !((args.len()==2 && args[0]!="history") || (args.len()==3 && args[0]=="history")) {
                    return RuleExpression::Err(format!("wrong number of arguments in {} args:{}",api, args.len()));
                }
                let pos = unwrap_or_return!(self.match_and_get_position(args, context), RuleExpression::Err(format!("could not get position in {}", api)));

                let mut kill = false;
                if let Some(ref piece2) = self.board[pos.0 as usize][pos.1 as usize] {
                    //TODO add if piece can be killed, for example if i can kill my own field
                    match piece2.owner {
                        Player::White => self.cementaries.0.push(piece2.clone()), // White's cemetery
                        Player::Black => self.cementaries.1.push(piece2.clone()), // Black's cemetery
                    }
                    kill = true;
                }

                if kill { evaluate_rules!(self, (pos.0, pos.1), deathrattle, context, depth, sender, receiver ); } //TODO maybe add some way to block deathrattle???
                self.board[pos.0 as usize][pos.1 as usize] = None;
                RuleExpression::Void
            }

            // args: [piece name ] piece corresponds to id field in piece(zmień płeć)
            "piece_on_board" => {
                if !self.pieces.contains_key(args[0]){
                    return RuleExpression::Err(format!("Piece {} does not exist in this game, in {}", args[0], api));
                }

                RuleExpression::Boolean(self.board.iter().any(|x| x.iter().any(|y| y.as_ref().is_some_and(|z| z.id==args[0]))))
            },

            "piece_on_board_cnt" => {
                if !self.pieces.contains_key(args[0]){
                    return RuleExpression::Err(format!("Piece {} does not exist in this game, in {}", args[0], api));
                }

                RuleExpression::Integer(self.board.iter().flat_map(|row| row.iter()).filter(|&piece| piece.as_ref().is_some_and(|v| v.id==args[0])).count() as i32)
            },
            // args: [piece name] [player]
            "player_piece_on_board" =>{
                if !self.pieces.contains_key(args[0]){
                    return RuleExpression::Err(format!("Piece {} does not exist in this game, in {}", args[0], api));
                }

                let pp;
                if args.len() > 1{
                    pp = unwrap_or_return!(Player::from_str(args[1]), RuleExpression::Err("Please add a valid player".to_string()));
                }
                else {
                    pp = self.current_player
                }

                RuleExpression::Boolean(self.board.iter().any(|x| x.iter().any(|y| y.as_ref().is_some_and(|z| z.id==args[0] && z.owner==pp))))
            },

            "player_piece_on_board_cnt" => {
                if !self.pieces.contains_key(args[0]){
                    return RuleExpression::Err(format!("Piece {} does not exist in this game, in {}", args[0], api));
                }
                let pp;
                if args.len() > 1{
                    pp = unwrap_or_return!(Player::from_str(args[1]), RuleExpression::Err("Please add a valid player".to_string()));
                }
                else {
                    pp = self.current_player
                }
                RuleExpression::Integer(self.board.iter().flat_map(|row| row.iter()).filter(|&piece| piece.as_ref().is_some_and(|v| v.id==args[0] && v.owner==pp)).count() as i32)
            },

            //args: position1 position2 position1 must have a piece and position2 must be empty
            "move_piece" => {
                //TODO add modifiers so it can beat and whether it can kill allies
                if args.len()!=2 {
                    return RuleExpression::Err("wrong number of arguments".to_string());
                }
                let pos1 = unwrap_or_return!(self.get_position(args[0]), RuleExpression::Err("position 1 is not valid".to_string()));
                let pos2 = unwrap_or_return!(self.get_position(args[1]), RuleExpression::Err("position 2 is not valid".to_string()));

                if self.board[pos2.0 as usize][pos2.1 as usize].is_some(){
                    return RuleExpression::Err("moving a piece to occupied position".to_string());
                }

                let piece = unwrap_or_return!(self.board[pos1.0 as usize][pos1.1 as usize].clone(), RuleExpression::Err("No piece at given position".to_string()));

                self.board[pos2.0 as usize][pos2.1 as usize] = Some(piece);
                self.board[pos1.0 as usize][pos1.1 as usize] = None;
                RuleExpression::Void
            },
            //args: position1 position2 [piece|owner]
            "is_same_line" => {
                if args.len()!=3 {
                    return RuleExpression::Err("wrong number of arguments".to_string());
                }

                let pos1 = unwrap_or_return!(self.get_position(args[0]), RuleExpression::Err("position 1 is not valid".to_string()));
                let pos2 = unwrap_or_return!(self.get_position(args[1]), RuleExpression::Err("position 1 is not valid".to_string()));

                let x = (pos2.0 as i32) - (pos1.0 as i32);
                let y = (pos2.1 as i32) - (pos1.1 as i32);

                if x!=0 && y!=0 && x.abs()!=y.abs(){
                    return RuleExpression::Err("Positions not on strainght line or a square diagonal".to_string());
                }

                if x==0 && y==0{
                    return RuleExpression::Boolean(true);
                }
                let diff = (x/std::cmp::max(x.abs(),y.abs()), y/std::cmp::max(x.abs(),y.abs()));
                let mut i = 0;
                let mut pos3 = pos1;
                //println!("polj: {:?}, {:?}, p3 {:?}, p1 {:?}, p2 {:?}, {}",diff,i,pos3,pos1, pos2, args[2]);
                while  pos3 != pos2 {
                   // println!("a: {:?}, {:?}, p3 {:?}, p1 {:?}, p2 {:?}, {}",diff,i,pos3,pos1, pos2, args[2]);
                    if self.board[pos3.0 as usize][pos3.1 as usize].is_none() && self.board[pos2.0 as usize][pos2.1 as usize].is_none(){
                        return RuleExpression::Boolean(false);

                    }
                    if self.board[pos3.0 as usize][pos3.1 as usize].is_some() && self.board[pos2.0 as usize][pos2.1 as usize].is_some(){
                        if match args[2] {
                            "piece" => {
                                self.board[pos3.0 as usize][pos3.1 as usize].as_ref().unwrap().id != self.board[pos2.0 as usize][pos2.1 as usize].as_ref().unwrap().id
                            }
                            "owner" => {
                                self.board[pos3.0 as usize][pos3.1 as usize].as_ref().unwrap().owner != self.board[pos2.0 as usize][pos2.1 as usize].as_ref().unwrap().owner
                            }
                            _ =>{
                                return RuleExpression::Err("unknown argument".to_string());
                            }
                        }{
                            return RuleExpression::Boolean(false);
                        }
                    }
                    else{
                        return RuleExpression::Boolean(false);
                    }


                    i +=1;
                    pos3 = (((pos1.0 as i32) + i*diff.0) as u32, ((pos1.1 as i32)+ i*diff.1) as u32);
                    //println!("e {:?}, p3 {:?}",i, pos3);
                }
                RuleExpression::Boolean(true)
            }
            not_found => {
                RuleExpression::Err(format!("nieznane zapytanie: {}", not_found))
            }
        }
    }

    ///zamienia pozycję z &str na (u32,u32)
    pub fn get_position(&self, pos: &str) -> Option<(u32, u32)> {
        self.parse_position(&pos)
    }

    ///zmienia pozyscje pos1 z &str na (u32,u32). przy tym jest względna
    pub fn get_position_from_relative(&self, pos1: &str, pos2: (u32, u32)) -> Option<(u32, u32)>{
        let p = unwrap_or_return!(self.parse_position(pos1), None);
        Some(((p.0 + pos2.0) as u32, (p.1 + pos2.1) as u32))
    }

    pub fn get_game_state(&mut self, context: &HashMap<String,String>) -> Result<GameState, String>{
        let mut ec = false;
        if self.endcondition.is_some(){
            let end =  self.endcondition.as_ref().unwrap().clone().evaluate(self, &context, 0, None, None);
            ec = true;
            match end {
                RuleExpression::Boolean(x) => {
                    if !x {
                      //  println!("no end");
                        self.current_player =  !self.current_player;
                        return Ok(GameState::Continue);
                    }
                },
                _ => {
                    return Err("In make move: sth went wrong, end result not bool".to_string());
                }
            }
        }

        let white: RuleExpression = self.wincondition.0.clone().evaluate(self, &context, 0, None, None);
        let black: RuleExpression = self.wincondition.1.clone().evaluate(self, &context, 0, None, None);
        match (white, black) {
            (RuleExpression::Boolean(w), RuleExpression::Boolean(b)) =>{
                if w == b {
                    if ec || w {
                        return Ok(GameState::Draw);
                    }

                    self.current_player =  !self.current_player;
                    return Ok(GameState::Continue);

                }
                if w {
                    return Ok(GameState::WhiteWins);
                }
                return Ok(GameState::BlackWins);
            }
            _ => {
                return Err("sth went wrong, one of the winconditions not bool".to_string());
            }
        }
    }

    pub fn make_move(&mut self, old_position: (u32, u32), new_position: (u32, u32), pmove: PMove, sender: &Sender<Option<String>>, receiver: &Receiver<String>) -> Result<GameState, String>{
        let (x1, y1) = old_position;
        let (x2, y2) = new_position;
        if x1 >= self.size.0 || y1 >= self.size.1 || x2 >= self.size.0 || y2 >= self.size.1
        {
            return Err("Out of board".to_string());
        }
        if self.board[x1 as usize][y1 as usize].is_none() {
             return Err("Piece doesn't exist".to_string());
        }

        let mut context = HashMap::new();
        context.insert(String::from("new_position"),format!("{},{}",x2,y2).to_string());
        context.insert(String::from("old_position"),format!("{},{}",x1,y1).to_string());


        evaluate_rules!(self, (x1, y1), onmove, context, 0, Some(sender), Some(receiver));
        let mut kill = false;
        if let Some(ref piece2) = self.board[x2 as usize][y2 as usize] {
            //TODO add if piece can be killed, for example if i can kill my own field
            match piece2.owner {
                Player::White => self.cementaries.0.push(piece2.clone()), // White's cemetery
                Player::Black => self.cementaries.1.push(piece2.clone()), // Black's cemetery
            }
            kill = true;
        }
        else {
            // leave it for now since we might want to do actions only if not kill
        }
        if kill { evaluate_rules!(self, (x2, y2), deathrattle, context, 0, Some(sender), Some(receiver)); }

        let temp = std::mem::take(&mut self.board[x1 as usize][y1 as usize]);
        self.board[x2 as usize][y2 as usize] = temp;
        self.board[x1 as usize][y1 as usize] = None;

        if kill { evaluate_rules!(self, (x2, y2), onkill, context, 0, Some(sender), Some(receiver));}


        if let Some(ref cons) = pmove.consequences {
            cons.iter().for_each(|x| {let _ = x.evaluate(self, &context, 0, Some(sender), Some(receiver));});
        }

        evaluate_rules!(self, (x2, y2), aftermove, context, 0, Some(sender), Some(receiver));

        self.add_history(pmove.clone(), old_position, new_position);

        self.get_game_state(&context)
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let string = self.board.iter()
                .map(|row| {
                    row.iter()
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
