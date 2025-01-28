#[cfg(test)]
mod tests {
    use boardgames::{player::Player, *};
    use boardgames::ruleexpression::RuleExpression;
    use std::collections::HashMap;
    use std::sync::mpsc::{channel, Receiver, Sender};


    #[test]
    fn test_board_initialization_and_piece_placement() {
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();

        let mut board = Board::new();
        board.endcondition = Some(RuleExpression::Boolean(false));

        let rook_json = std::fs::read_to_string("res/chess/rook.json").ok().unwrap();
        let rook: Piece = Piece::new(rook_json.to_string()).expect("Failed to create rook");
        board.board[4][5] = Some(rook);

        let ligma: Piece = Piece::new(r#"{"id":"ligma","name":"Ligma","owner":"Black","possiblemoves": []}"#.to_string()).expect("Failed to create ligma piece");
        board.board[2][5] = Some(ligma);

        let possible_moves = board.get_possible_positions((4, 5), 0).unwrap();
        assert!(!possible_moves.is_empty(), "Rook should have possible moves");

        assert!(board.make_move((4, 5), (2, 5), board.board[4][5].as_ref().unwrap().possiblemoves[0].clone(), &tx, &rx2).is_ok());
    }

    #[test]
    fn test_board_from_json(){
        let board_json = std::fs::read_to_string("res/chess/szachy.json").ok().unwrap();
        let board = Board::from_json(board_json.to_string());

        assert!(board.is_ok());
        let board2:Board = board.unwrap();
        assert!(board2.board[0][1].is_some() && board2.board[1][0].is_some(), "there should be pieces and those fiedls");
    }

    #[test]
    fn test_cementary(){
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();

        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        pawn.set_owner(Player::Black);
        board.board[1][1] = Some(pawn);

        let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[2][2] = Some(pawn2.clone());

        let _= board.make_move((2,2),(1,1) ,pawn2.possiblemoves[2].clone(),&tx,&rx2);
        assert!(board.board[2][2].is_none(), "this field should be empty");
        assert!(board.cementaries.1.len()==1,"cementary shoud have one piece");
        assert!(board.cementaries.1[0].id=="pawn_1" && board.cementaries.1[0].owner==Player::Black);
    }

    #[test]
    fn test_player_change(){
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new(); //TODO ADD REASONABLE CONDITIONS IN BOARD
        board.endcondition = Some(RuleExpression::Boolean(false));

        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        pawn.set_owner(Player::Black);

        board.board[1][1] = Some(pawn);
        let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[2][2] = Some(pawn2.clone());

        assert!(board.current_player==Player::White, "current_player player should be white");
        let x = board.make_move((2,2),(1,1) ,pawn2.possiblemoves[2].clone(),&tx,&rx2);
        println!("{:?}",x);
        assert!(board.current_player==Player::Black, "current_player player should be black");
    }

    #[test]
    fn test_knight_moves() {
        let mut board = Board::new();

        let knight_json = std::fs::read_to_string("res/chess/knight.json").ok().unwrap();
        let knight: Piece = Piece::new(knight_json.to_string()).expect("Failed to create knight");
        board.board[3][2] = Some(knight);

        let possible_moves = board.get_possible_positions((3, 2), 0).unwrap();
        assert!(possible_moves.len()==8, "knight should have 8 moves now");

        let knight2: Piece = Piece::new(knight_json.to_string()).expect("Failed to create knight");
        board.board[5][3] = Some(knight2);

        let possible_moves = board.get_possible_positions((3, 2), 0).unwrap();
        assert!(possible_moves.len()==7, "knight should have 7 moves now (no friendly fire)");
    }

    #[test]
    fn test_bishop_moves() {
        let mut board = Board::new();

        let bishop_json = std::fs::read_to_string("res/chess/bishop.json").ok().unwrap();
        let bishop: Piece = Piece::new(bishop_json.to_string()).expect("Failed to create bishop");
        board.board[6][4] = Some(bishop);

        let possible_moves = board.get_possible_positions((6, 4), 0).unwrap();
        assert!(!possible_moves.is_empty(), "Bishop should have possible moves");
    }

    #[test]
    fn test_pawn_moves() {
        let mut board = Board::new();

        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[1][1] = Some(pawn.clone());

        let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[2][2] = Some(pawn2);

        let mut context = HashMap::new();
        context.insert(String::from("old_position"),format!("{},{}",1,1).to_string());
        context.insert(String::from("new_position"),format!("{},{}",0,1).to_string());

        let possible_moves = board.get_possible_positions((1, 1), 0).unwrap();
        assert!(!possible_moves.is_empty(), "Pawn should have possible moves");
        assert!(possible_moves.len()==1, "Pawn should have 1 move");

        let possible_moves = board.get_possible_positions((2, 2), 0).unwrap();
        assert!(possible_moves.len()==2, "Pawn should have 2 moves");

        pawn.set_owner(Player::Black);
        board.board[1][1] = Some(pawn); board.current_player=Player::Black;
        let possible_moves = board.get_possible_positions((1, 1), 0).unwrap();
        println!("{:?}", possible_moves);
        println!("{:?}",board.board[2][2].as_ref().unwrap().owner);
        assert!(possible_moves.len()==3, "Pawn should have 3 moves");
        board.current_player=Player::White;
        let possible_moves = board.get_possible_positions((2, 2), 0).unwrap();
        assert!(possible_moves.len()==3, "Pawn should have 3 moves");

    }

    #[test]
    fn test_en_passant(){
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.current_player = Player::Black;

        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        pawn.set_owner(Player::Black);
        board.board[1][1] = Some(pawn.clone());

        let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[3][2] = Some(pawn2);

        let _ = board.make_move((1,1),(3,1) ,board.board[1][1].as_ref().unwrap().possiblemoves[0].clone(),&tx, &rx2);

        let possible_moves = board.get_possible_positions((3, 2), 0).unwrap();
        assert!(!possible_moves.is_empty(), "Pawn should have possible moves");
        assert!(possible_moves.len()==3, "Pawn should have 3 move");

        let res = board.make_move((3,2),(2,1) ,board.board[3][2].as_ref().unwrap().possiblemoves[3].clone(),&tx,&rx2);

        println!("{:?}",res);
        assert!(res.is_ok(), "move should succeed");

        let board_str = format!("{}", board);
        let board_lines = board_str.lines().map(|line| line.to_string()).collect::<Vec<String>>();
        assert!(board.board[3][1].is_none());
        assert!(board_lines[2]==". p . . . . . .");
        assert!(board_lines[3]==". . . . . . . .");
    }

    #[test]
    fn test_is_attacked(){
        let mut board = Board::new();

        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        pawn.set_owner(Player::Black);
        board.board[1][1] = Some(pawn);

        let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[2][2] = Some(pawn2);

        let attackers = board.get_attackers((1,1), 0);
        assert!(attackers.is_some_and( |attackers| attackers.len()==1),"Pawn should be attacked by one piece");
    }

    #[test]
    fn test_can_attack(){
        let mut board = Board::new();

        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        pawn.set_owner(Player::Black);
        board.board[1][1] = Some(pawn);

        let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[2][2] = Some(pawn2);

        let attacked = board.get_attacked((2,2), 0);
        assert!(attacked.is_some_and(|x| x.len()==1), "Pawn should be able to attack one field");
    }

    #[test]
    fn test_board_loading() {
        let board_json = std::fs::read_to_string("res/chess/szachy.json").ok().unwrap();
        let board_result = Board::from_json(board_json.to_string());

        assert!(board_result.is_ok(), "Board should be successfully loaded from JSON");
    }

    #[test]
    fn test_piece_on_board(){
        let mut board = Board::new();

        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        let _ = board.add_piece(pawn_json.clone());
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[1][1] = Some(pawn);
        let context = HashMap::new();
        assert!(board.call_api("piece_on_board", vec!["pawn_1"] ,&context,0, None, None )==RuleExpression::Boolean(true), "there sould be one pawn on the board");
        assert!(board.call_api("piece_on_board_cnt", vec!["pawn_1"] ,&context, 0, None, None )==RuleExpression::Integer(1), "there sould be one pawn on the board");
        assert!(board.call_api("player_piece_on_board", vec!["pawn_1", "White"] ,&context, 0, None, None )==RuleExpression::Boolean(true), "there sould be one white pawn on the board");
        assert!(board.call_api("player_piece_on_board", vec!["pawn_1", "Black"] ,&context, 0, None, None )==RuleExpression::Boolean(false), "no black pawns on the board");
        assert!(board.call_api("player_piece_on_board_cnt", vec!["pawn_1", "Black"] ,&context, 0, None, None )==RuleExpression::Integer(0), "no black pawns on the board");
        assert!(board.call_api("player_piece_on_board_cnt", vec!["pawn_1", "htrfd"] ,&context, 0, None, None )==RuleExpression::Err("Please add a valid player".to_string()), "no such player");
    }

    #[test]
    fn test_boardend1(){
        //end = true, black wins
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();

        board.endcondition = Some(RuleExpression::Boolean(true));
        board.wincondition.1 = RuleExpression::Boolean(true);
        board.wincondition.0 = RuleExpression::Boolean(false);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(), &tx, &rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::BlackWins);
    }

    #[test]
    fn test_boardend2(){
        //end = true, draw
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = Some(RuleExpression::Boolean(true));
        board.wincondition.0 = RuleExpression::Boolean(true);
        board.wincondition.1 = RuleExpression::Boolean(true);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(), &tx, &rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::Draw);
    }

    #[test]
    fn test_boardend3(){
        //end = true, white wins
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = Some(RuleExpression::Boolean(true));
        board.wincondition.0 = RuleExpression::Boolean(true);
        board.wincondition.1 = RuleExpression::Boolean(false);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),&tx,&rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::WhiteWins);
    }

    #[test]
    fn test_boardend4(){
        //end = true, continue
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = Some(RuleExpression::Boolean(true));
        board.wincondition.0 = RuleExpression::Boolean(false);
        board.wincondition.1 = RuleExpression::Boolean(false);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),&tx, &rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::Draw);
    }

    #[test]
    fn test_boardend5(){
        //end = null, black wins
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = None;
        board.wincondition.0 = RuleExpression::Boolean(false);
        board.wincondition.1 = RuleExpression::Boolean(true);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),&tx, &rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::BlackWins);
    }

    #[test]
    fn test_boardend6(){
        //end = null, draw
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = None;
        board.wincondition.0 = RuleExpression::Boolean(true);
        board.wincondition.1 = RuleExpression::Boolean(true);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(), &tx, &rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::Draw);
    }

    #[test]
    fn test_boardend7(){
        //end = null, white wins
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = None;
        board.wincondition.0 = RuleExpression::Boolean(true);
        board.wincondition.1 = RuleExpression::Boolean(false);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(), &tx, &rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::WhiteWins);
    }

    #[test]
    fn test_boardend8(){
        //end = null, continue
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = None;
        board.wincondition.0 = RuleExpression::Boolean(false);
        board.wincondition.1 = RuleExpression::Boolean(false);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(), &tx, &rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::Continue);
    }

    #[test]
    fn test_boardend9(){
        //end = false, black wins
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = Some(RuleExpression::Boolean(false));
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        board.add_piece(pawn_json.clone()).ok();
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(), &tx, &rx2);
        assert!(x.is_ok());
        assert!(x.unwrap()==GameState::Continue);
    }

    #[test]
    fn test_king_moves() {
        let mut board = Board::new();

        let king_json = std::fs::read_to_string("res/chess/king.json").ok().unwrap();
        let king: Piece = Piece::new(king_json.to_string()).expect("Failed to create king");
        board.board[1][1] = Some(king.clone());

        let possible_moves = board.get_possible_positions((1, 1), 0).unwrap();
        println!("{:?}",possible_moves );
        assert!(!possible_moves.is_empty(), "king should have possible moves");
        assert!(possible_moves.len()==8, "King should have 8 move");
    }

    #[test]
    fn test_short_castling(){
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = Some(RuleExpression::Boolean(false)); //by endconditoin nie dostał wylewu

        let king_json = std::fs::read_to_string("res/chess/king.json").ok().unwrap();
        let mut king: Piece = Piece::new(king_json.to_string()).expect("Failed to create pawn");
        board.board[7][4] = Some(king.clone());
        king.set_owner(Player::Black);
        board.board[0][4] = Some(king.clone());

        let rook_json = std::fs::read_to_string("res/chess/rook.json").ok().unwrap();
        let mut rook: Piece = Piece::new(rook_json.to_string()).expect("Failed to create rook");

        board.board[7][7] = Some(rook.clone());
        rook.set_owner(Player::Black);
        board.board[0][7] = Some(rook.clone());

        let possible_moves = board.get_possible_positions((7, 4), 0).unwrap();

        assert!(!possible_moves.is_empty(), "king should have possible moves");
        assert!(possible_moves.len()==6, "King should have 6 moves");
        assert!(possible_moves.contains(&(7u32,6u32)), "king shoul castle");
        let res = board.make_move((7u32,4u32),(7u32,6u32), king.possiblemoves[1].clone(), &tx, &rx2);
        println!("{:?}", res);
        assert!(res.is_ok(), "castling should succeed");
        let board_str = format!("{}", board);
        let board_lines = board_str.lines().map(|line| line.to_string()).collect::<Vec<String>>();
        assert!(board_lines[7]==". . . . . r k .");
        println!("{:?}", board_lines);

        board.board[7][5] = None;

        board.current_player=Player::Black;
        //assert if black can also castle
        let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
        assert!(!possible_moves.is_empty(), "king should have possible moves");
        println!("{:?}", possible_moves);


        assert!(possible_moves.len()==6, "King should have 6 moves");
        assert!(possible_moves.contains(&(0u32,6u32)), "king shouldn castle");


        //assert that blocking prevents castling
        rook.set_owner(Player::White);
        println!("1\n{}", board);
        board.board[1][6] = Some(rook.clone());

        let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
        println!("2\n{}", board);
        println!("{:?}", possible_moves);
        assert!(possible_moves.len()==5, "King should have 5 moves");
        assert!(!possible_moves.contains(&(0u32,6u32)), "king should not castle");

        board.board[1][6] = None;
        board.board[1][5] = Some(rook.clone());

        let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
        assert!(possible_moves.len()==5, "King should have 5 moves");
        assert!(!possible_moves.contains(&(0u32,6u32)), "king should not castle");

        board.board[1][5] = None;


        //assert move_count prevets castling
        board.board[0][7].clone().unwrap().memory.unwrap().insert("move_count".to_string(), RuleExpression::Integer(1));

        assert!(possible_moves.len()==5, "King should have 5 moves");
        assert!(!possible_moves.contains(&(0u32,6u32)), "king should not castle");

        board.board[0][7].clone().unwrap().memory.unwrap().insert("move_count".to_string(), RuleExpression::Integer(0));
        board.board[0][4].clone().unwrap().memory.unwrap().insert("move_count".to_string(), RuleExpression::Integer(1));
        board.board[0][7].clone().unwrap().memory.unwrap().insert("move_count".to_string(), RuleExpression::Integer(1));

        assert!(possible_moves.len()==5, "King should have 5 moves");
        assert!(!possible_moves.contains(&(0u32,6u32)), "king should not castle");
    }

    #[test]
    fn test_long_castling(){
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.endcondition = Some(RuleExpression::Boolean(false)); //by endconditoin nie dostał wylewu

        let king_json = std::fs::read_to_string("res/chess/king.json").ok().unwrap();
        let mut king: Piece = Piece::new(king_json.to_string()).expect("Failed to create pawn");
        board.board[7][4] = Some(king.clone());
        king.set_owner(Player::Black);
        board.board[0][4] = Some(king.clone());

        let rook_json = std::fs::read_to_string("res/chess/rook.json").ok().unwrap();
        let mut rook: Piece = Piece::new(rook_json.to_string()).expect("Failed to create rook");

        board.board[7][0] = Some(rook.clone());
        rook.set_owner(Player::Black);
        board.board[0][0] = Some(rook.clone());

        let possible_moves = board.get_possible_positions((7, 4), 0).unwrap();

        assert!(!possible_moves.is_empty(), "king should have possible moves");
        assert!(possible_moves.len()==6, "King should have 6 moves");
        assert!(possible_moves.contains(&(7u32,2u32)), "king shoul castle");
        let res = board.make_move((7u32,4u32),(7u32,2u32), king.possiblemoves[2].clone(), &tx, &rx2);
        assert!(res.is_ok(), "castling should succeed");
        let board_str = format!("{}", board);
        let board_lines = board_str.lines().map(|line| line.to_string()).collect::<Vec<String>>();
        println!("{:?}", board_lines);
        assert!(board_lines[7]==". . k r . . . .", "board after castle");


        board.board[7][3] = None;

        board.current_player=Player::Black;
        //assert if black can also castle
        let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
        assert!(!possible_moves.is_empty(), "king should have possible moves");
        println!("{:?}", possible_moves);


        assert!(possible_moves.len()==6, "King should have 6 moves");
        assert!(possible_moves.contains(&(0u32,2u32)), "king should castle");


        //assert that blocking prevents castling
        rook.set_owner(Player::White);
        board.board[0][1] = Some(rook.clone());

        let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
        assert!(possible_moves.len()==5, "King should have 5 moves, blocked path");
        assert!(!possible_moves.contains(&(0u32,2u32)), "king should not castle");

        board.board[0][1] = None;
        board.board[1][2] = Some(rook.clone());

        let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
        assert!(possible_moves.len()==5, "King should have 5 moves, field attacked1");
        assert!(!possible_moves.contains(&(0u32,2u32)), "king should not castle");

        board.board[1][2] = None;
        board.board[1][3] = Some(rook.clone());

        let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
        assert!(possible_moves.len()==5, "King should have 5 moves, field attacked2");
        assert!(!possible_moves.contains(&(0u32,2u32)), "king should not castle");
        board.board[1][3] = None;

        //assert move_count prevets castling
        board.board[0][0].clone().unwrap().memory.unwrap().insert("move_count".to_string(), RuleExpression::Integer(1));

        assert!(possible_moves.len()==5, "King should have 5 moves reason mc1");
        assert!(!possible_moves.contains(&(0u32,2u32)), "king should not castle");

        board.board[0][0].clone().unwrap().memory.unwrap().insert("move_count".to_string(), RuleExpression::Integer(0));
        board.board[0][4].clone().unwrap().memory.unwrap().insert("move_count".to_string(), RuleExpression::Integer(1));
        board.board[0][0].clone().unwrap().memory.unwrap().insert("move_count".to_string(), RuleExpression::Integer(1));

        assert!(possible_moves.len()==5, "King should have 5 moves reason mc2");
        assert!(!possible_moves.contains(&(0u32,2u32)), "king should not castle");
    }

    #[test]
    fn test_history(){
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();

        assert!(board.history.is_empty(), "history should have no entries");
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        let _ = board.add_piece(pawn_json.clone());
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn);
        let x = board.make_move((6, 6), (4, 6), board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),&tx,&rx2);
        assert!(x.is_ok());
        assert!(board.history.len()==1, "history should have 1 entry");
        assert!(board.history[0].1.1==(4,6), "wrong destination");
        assert!(board.history[0].1.0==(6,6), "wrong source");
        assert!(board.history[0].0 == board.board[4][6].as_ref().unwrap().possiblemoves[0].clone(), "wrong move");
    }

    #[test]
    fn test_depth_err(){
        let mut board = Board::new();
        let context = HashMap::new();
        let res = RuleExpression::Boolean(false).evaluate(&mut board,&context , 300, None, None);
        assert!(res == RuleExpression::Err("too many calls".to_string()))
    }

    #[test]
    fn test_revive(){
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();
        board.revive = RuleExpression::Boolean(true);
        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        let _ = board.add_piece(pawn_json.clone());
        let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");

        board.cementaries.0.push(pawn.clone());
        let res = board.revive_piece(true,0 ,(1,2),&tx, &rx2 );
        println!("{:?}", res);
        assert!(res.is_ok(), "revive should succeed");
        assert!(board.board[1][2].is_some(), "there should be piece");
        assert!(board.board[1][2].clone().unwrap().id=="pawn_1".to_string());
    }

    #[test]
    fn test_is_same_line(){
        let (tx, _):(Sender<Option<String>>, Receiver<Option<String>>) = channel();
        let (_, rx2):(Sender<String>, Receiver<String>) = channel();
        let mut board = Board::new();

        let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
        let _ = board.add_piece(pawn_json.clone());
        let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
        board.board[6][6] = Some(pawn.clone());
        pawn.owner=Player::Black;
        board.board[6][7] = Some(pawn);
        let king_json = std::fs::read_to_string("res/chess/king.json").ok().unwrap();
        let king: Piece = Piece::new(king_json.to_string()).expect("Failed to create pawn");
        board.board[7][7] = Some(king.clone());

        let context = HashMap::new();
        let res = board.call_api("is_same_line", vec!["6,6","7,7","owner"], &context, 0, Some(&tx),Some(&rx2) );
        assert!(res==RuleExpression::Boolean(true), "same owner");
        let res = board.call_api("is_same_line", vec!["6,6","7,7","piece"], &context, 0, Some(&tx),Some(&rx2) );
        assert!(res==RuleExpression::Boolean(false), "different piece");
        let res = board.call_api("is_same_line", vec!["6,6","6,7","piece"], &context, 0, Some(&tx),Some(&rx2) );
        assert!(res==RuleExpression::Boolean(true), "same piece");
        let res = board.call_api("is_same_line", vec!["6,6","6,7","owner"], &context, 0, Some(&tx),Some(&rx2) );
        assert!(res==RuleExpression::Boolean(false), "different owner");
    }

    #[test]
    fn test_x_moves() {
        let mut board = Board::new();

        let x_json = std::fs::read_to_string("res/tictactoe/x.json").ok().unwrap();
        let x: Piece = Piece::new(x_json.to_string()).expect("Failed to create x");
        board.board[0][4] = Some(x.clone());

        println!("{:?}",board.get_possible_positions((0, 4), 0) );
        let possible_moves = board.get_possible_positions((0, 4), 0);

        assert!(possible_moves.is_some(), "x should have possible moves");
        assert!(possible_moves.unwrap().len()==9, "aaa");
    }
}
