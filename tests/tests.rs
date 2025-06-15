#[cfg(test)]
mod tests {
  use boardgames::ruleexpression::RuleExpression;
  use boardgames::{player::Player, *};
  use std::collections::HashMap;
  use std::sync::mpsc::{channel, Receiver, Sender};

  #[test]
  fn test_board_from_json() {
    //TODO change this test
    // curretly if changed to szachy it would fail because no ids are given when reading a board
    let board_json = std::fs::read_to_string("res/chess/szachy.json")
      .ok()
      .unwrap();
    let board = Board::from_json(board_json.to_string());

    assert!(board.is_ok());
    let board2: Board = board.unwrap();
    assert!(
      board2.board[0][1].is_some() && board2.board[1][0].is_some(),
      "there should be pieces and those fiedls"
    );
    assert!(
      <Option<boardgames::Piece> as Clone>::clone(&board2.board[0][1])
        .is_some_and(|y| y.memory["id"] == RuleExpression::Integer(1))
    );
  }

  #[test]
  fn test_shorthands() {
    //TODO change this test
    // curretly if changed to szachy it would fail because no ids are given when reading a board
    let board_json = std::fs::read_to_string("res/examples/init_example.json")
      .ok()
      .unwrap();
    let board = Board::from_json(board_json.to_string());

    assert!(board.is_ok());
    let board2: Board = board.unwrap();
    assert!(
      board2.board[0][1].is_some(),
      "there should be a piece on that field"
    );
    assert!(
      <Option<boardgames::Piece> as Clone>::clone(&board2.board[0][1])
        .is_some_and(|x| x.memory["id"] == RuleExpression::Integer(10))
    );
    println!("{:?}", board2.cementaries);
    assert!(
      board2.cementaries.0.len() > 0 && board2.cementaries.1.len() > 0,
      "there should be pieces on both cementaries"
    );
    assert!(board2.cementaries.0[0].memory["id"] == RuleExpression::Integer(1));
    assert!(board2.cementaries.1[0].memory["id"] == RuleExpression::Integer(6));
  }

  #[test]
  fn test_cementary() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    pawn.set_owner(Player::Black);
    board.board[1][1] = Some(pawn);

    let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[2][2] = Some(pawn2.clone());

    let _ = board.make_move(
      (2, 2),
      (1, 1),
      pawn2.possiblemoves[2].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(board.board[2][2].is_none(), "this field should be empty");
    assert!(
      board.cementaries.0.len() == 1,
      "cementary shoud have one piece"
    );
    assert!(board.cementaries.0[0].id == "pawn_1" && board.cementaries.0[0].owner == Player::White);
  }

  #[test]
  fn test_player_change() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new(); //TODO ADD REASONABLE CONDITIONS IN BOARD
    board.endcondition = Some(RuleExpression::Boolean(false));

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    pawn.set_owner(Player::Black);

    board.board[1][1] = Some(pawn);
    let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[2][2] = Some(pawn2.clone());

    assert!(
      board.current_player == Player::White,
      "current_player player should be white"
    );
    let x = board.make_move(
      (2, 2),
      (1, 1),
      pawn2.possiblemoves[2].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    println!("{:?}", x);
    assert!(
      board.current_player == Player::Black,
      "current_player player should be black"
    );
  }

  #[test]
  fn test_knight_moves() {
    let mut board = Board::new();

    let knight_json = std::fs::read_to_string("res/chess/knight.json")
      .ok()
      .unwrap();
    let knight: Piece = Piece::new(knight_json.to_string()).expect("Failed to create knight");
    board.board[3][2] = Some(knight);

    let possible_moves = board.get_possible_positions((3, 2), 0).unwrap();
    assert!(possible_moves.len() == 8, "knight should have 8 moves now");

    let knight2: Piece = Piece::new(knight_json.to_string()).expect("Failed to create knight");
    board.board[5][3] = Some(knight2);

    let possible_moves = board.get_possible_positions((3, 2), 0).unwrap();
    assert!(
      possible_moves.len() == 7,
      "knight should have 7 moves now (no friendly fire)"
    );
  }

  #[test]
  fn test_bishop_moves() {
    let mut board = Board::new();

    let bishop_json = std::fs::read_to_string("res/chess/bishop.json")
      .ok()
      .unwrap();
    let bishop: Piece = Piece::new(bishop_json.to_string()).expect("Failed to create bishop");
    board.board[6][4] = Some(bishop);

    let possible_moves = board.get_possible_positions((6, 4), 0).unwrap();
    assert!(
      !possible_moves.is_empty(),
      "Bishop should have possible moves"
    );
  }

  #[test]
  fn test_friendly_fire() {
    let mut board = Board::new();

    let bishop_json = std::fs::read_to_string("res/chess/bishop.json")
      .ok()
      .unwrap();
    let bishop: Piece = Piece::new(bishop_json.to_string()).expect("Failed to create bishop");
    board.board[6][4] = Some(bishop);

    let queen_json = std::fs::read_to_string("res/chess/queen.json")
      .ok()
      .unwrap();
    let queen: Piece = Piece::new(queen_json.to_string()).expect("Failed to create queen");
    board.board[5][3] = Some(queen);

    let possible_moves = board.get_possible_positions((6, 4), 0).unwrap();
    let possible_moves2 = board.get_possible_positions((5, 3), 0).unwrap();
    assert!(
      !possible_moves.contains(&(5, 3)),
      "bishop should not kill queen"
    );
    assert!(
      !possible_moves2.contains(&(6, 4)),
      "queen should not kill bishop"
    );
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
    context.insert(
      String::from("old_position"),
      format!("{},{}", 1, 1).to_string(),
    );
    context.insert(
      String::from("new_position"),
      format!("{},{}", 0, 1).to_string(),
    );

    let possible_moves = board.get_possible_positions((1, 1), 0).unwrap();
    assert!(
      !possible_moves.is_empty(),
      "Pawn should have possible moves"
    );
    assert!(possible_moves.len() == 1, "Pawn should have 1 move");

    let possible_moves = board.get_possible_positions((2, 2), 0).unwrap();
    assert!(possible_moves.len() == 2, "Pawn should have 2 moves");

    pawn.set_owner(Player::Black);
    board.board[1][1] = Some(pawn);
    board.current_player = Player::Black;
    let possible_moves = board.get_possible_positions((1, 1), 0).unwrap();
    println!("{:?}", possible_moves);
    println!("{:?}", board.board[2][2].as_ref().unwrap().owner);
    assert!(possible_moves.len() == 3, "Pawn should have 3 moves");
    board.current_player = Player::White;
    let possible_moves = board.get_possible_positions((2, 2), 0).unwrap();
    assert!(possible_moves.len() == 3, "Pawn should have 3 moves");
  }

  #[test]
  fn test_en_passant() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    let mut pawn2 = pawn.clone();
    pawn2.owner = Player::Black;

    board.board[7][0] = Some(pawn.clone());
    board.board[5][1] = Some(pawn2.clone());

    let _ = board.make_move(
      (7, 0),
      (5, 0),
      board.board[7][0].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );

    let possible_moves = board.get_possible_positions((5, 1), 0).unwrap();
    assert!(
      !possible_moves.is_empty(),
      "Pawn should have possible moves"
    );
    assert!(possible_moves.len() == 3, "Pawn should have 3 move");

    let res = board.make_move(
      (5, 1),
      (6, 0),
      board.board[5][0].as_ref().unwrap().possiblemoves[3].clone(),
      &tx,
      &rx2,
      true,
      false,
    );

    assert!(res.is_ok(), "move should succeed");

    board.board[7][2] = Some(pawn.clone());
    board.board[5][3] = Some(pawn2.clone());
    let _ = board.make_move(
      (7, 2),
      (5, 2),
      board.board[7][2].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );

    board.board[5][2].as_mut().unwrap().id = "eufh".to_string();
    let possible_moves = board.get_possible_positions((5, 3), 0).unwrap();
    assert!(
      !possible_moves.is_empty(),
      "Pawn should have possible moves"
    );
    assert!(possible_moves.len() == 2, "Pawn should have 3 move");
  }

  #[test]
  fn test_is_attacked() {
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    pawn.set_owner(Player::Black);
    board.board[1][1] = Some(pawn);

    let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[2][2] = Some(pawn2);

    let attackers = board.get_attackers((1, 1), 0);
    assert!(
      attackers.is_some_and(|attackers| attackers.len() == 1),
      "Pawn should be attacked by one piece"
    );
  }

  #[test]
  fn test_can_attack() {
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    pawn.set_owner(Player::Black);
    board.board[1][1] = Some(pawn);

    let pawn2: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[2][2] = Some(pawn2);

    let attacked = board.get_attacked((2, 2), 0);
    assert!(
      attacked.is_some_and(|x| x.len() == 1),
      "Pawn should be able to attack one field"
    );
  }

  #[test]
  fn test_board_loading() {
    let board_json = std::fs::read_to_string("res/chess/szachy.json")
      .ok()
      .unwrap();
    let board_result = Board::from_json(board_json.to_string());

    assert!(
      board_result.is_ok(),
      "Board should be successfully loaded from JSON"
    );
  }

  #[test]
  fn test_piece_on_board() {
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let _ = board.add_piece(pawn_json.clone());
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[1][1] = Some(pawn);
    let mut context = HashMap::new();
    assert!(
      board.call_api(
        "piece_on_board",
        vec!["pawn_1"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(true),
      "there sould be one pawn on the board"
    );
    assert!(
      board.call_api(
        "piece_on_board_cnt",
        vec!["pawn_1"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Integer(1),
      "there sould be one pawn on the board"
    );
    assert!(
      board.call_api(
        "player_piece_on_board",
        vec!["pawn_1", "White"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(true),
      "there sould be one white pawn on the board"
    );
    assert!(
      board.call_api(
        "player_piece_on_board",
        vec!["pawn_1", "Black"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(false),
      "no black pawns on the board"
    );
    assert!(
      board.call_api(
        "player_piece_on_board_cnt",
        vec!["pawn_1", "Black"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Integer(0),
      "no black pawns on the board"
    );
    assert!(
      board.call_api(
        "player_piece_on_board_cnt",
        vec!["pawn_1", "htrfd"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Err("Please add a valid player".to_string()),
      "no such player"
    );
  }

  #[test]
  fn test_boardend1() {
    //end = true, black wins
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();

    board.endcondition = Some(RuleExpression::Boolean(true));
    board.wincondition.1 = RuleExpression::Boolean(true);
    board.wincondition.0 = RuleExpression::Boolean(false);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let possible_moves = board.get_possible_positions((6, 6), 0).unwrap();
    println!("{:?}", possible_moves);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::BlackWins);
  }

  #[test]
  fn test_boardend2() {
    //end = true, draw
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.endcondition = Some(RuleExpression::Boolean(true));
    board.wincondition.0 = RuleExpression::Boolean(true);
    board.wincondition.1 = RuleExpression::Boolean(true);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::Draw);
  }

  #[test]
  fn test_boardend3() {
    //end = true, white wins
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.endcondition = Some(RuleExpression::Boolean(true));
    board.wincondition.0 = RuleExpression::Boolean(true);
    board.wincondition.1 = RuleExpression::Boolean(false);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::WhiteWins);
  }

  #[test]
  fn test_boardend4() {
    //end = true, continue
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.endcondition = Some(RuleExpression::Boolean(true));
    board.wincondition.0 = RuleExpression::Boolean(false);
    board.wincondition.1 = RuleExpression::Boolean(false);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::Draw);
  }

  #[test]
  fn test_boardend5() {
    //end = null, black wins
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.endcondition = None;
    board.wincondition.0 = RuleExpression::Boolean(false);
    board.wincondition.1 = RuleExpression::Boolean(true);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::BlackWins);
  }

  #[test]
  fn test_boardend6() {
    //end = null, draw
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.endcondition = None;
    board.wincondition.0 = RuleExpression::Boolean(true);
    board.wincondition.1 = RuleExpression::Boolean(true);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::Draw);
  }

  #[test]
  fn test_boardend7() {
    //end = null, white wins
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.endcondition = None;
    board.wincondition.0 = RuleExpression::Boolean(true);
    board.wincondition.1 = RuleExpression::Boolean(false);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::WhiteWins);
  }

  #[test]
  fn test_boardend8() {
    //end = null, continue
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.endcondition = None;
    board.wincondition.0 = RuleExpression::Boolean(false);
    board.wincondition.1 = RuleExpression::Boolean(false);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::Continue);
  }

  #[test]
  fn test_boardend9() {
    //end = false, black wins
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.endcondition = Some(RuleExpression::Boolean(false));
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(x.unwrap() == GameState::Continue);
  }

  #[test]
  fn test_king_moves() {
    let mut board = Board::new();

    let king_json = std::fs::read_to_string("res/chess/king.json").ok().unwrap();
    let king: Piece = Piece::new(king_json.to_string()).expect("Failed to create king");
    board.board[1][1] = Some(king.clone());

    let possible_moves = board.get_possible_positions((1, 1), 0).unwrap();
    println!("{:?}", possible_moves);
    assert!(
      !possible_moves.is_empty(),
      "king should have possible moves"
    );
    assert!(possible_moves.len() == 8, "King should have 8 move");
  }

  #[test]
  fn test_short_castling() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
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

    assert!(
      !possible_moves.is_empty(),
      "king should have possible moves"
    );
    assert!(possible_moves.len() == 6, "King should have 6 moves");
    assert!(possible_moves.contains(&(7u32, 6u32)), "king shoul castle");
    let res = board.make_move(
      (7u32, 4u32),
      (7u32, 6u32),
      king.possiblemoves[1].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    println!("{:?}", res);
    assert!(res.is_ok(), "castling should succeed");
    let board_str = format!("{}", board);
    let board_lines = board_str
      .lines()
      .map(|line| line.to_string())
      .collect::<Vec<String>>();
    assert!(board_lines[7] == ". . . . . r k .");
    println!("{:?}", board_lines);

    board.board[7][5] = None;

    board.current_player = Player::Black;
    //assert if black can also castle
    let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
    assert!(
      !possible_moves.is_empty(),
      "king should have possible moves"
    );
    println!("{:?}", possible_moves);

    assert!(possible_moves.len() == 6, "King should have 6 moves");
    assert!(
      possible_moves.contains(&(0u32, 6u32)),
      "king shouldn castle"
    );

    //assert that blocking prevents castling
    rook.set_owner(Player::White);
    println!("1\n{}", board);
    board.board[1][6] = Some(rook.clone());

    let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
    println!("2\n{}", board);
    println!("{:?}", possible_moves);
    assert!(possible_moves.len() == 5, "King should have 5 moves");
    assert!(
      !possible_moves.contains(&(0u32, 6u32)),
      "king should not castle"
    );

    board.board[1][6] = None;
    board.board[1][5] = Some(rook.clone());

    let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
    assert!(possible_moves.len() == 5, "King should have 5 moves");
    assert!(
      !possible_moves.contains(&(0u32, 6u32)),
      "king should not castle"
    );

    board.board[1][5] = None;

    //assert move_count prevets castling
    board.board[0][7]
      .clone()
      .unwrap()
      .memory
      .insert("move_count".to_string(), RuleExpression::Integer(1));

    assert!(possible_moves.len() == 5, "King should have 5 moves");
    assert!(
      !possible_moves.contains(&(0u32, 6u32)),
      "king should not castle"
    );

    board.board[0][7]
      .clone()
      .unwrap()
      .memory
      .insert("move_count".to_string(), RuleExpression::Integer(0));
    board.board[0][4]
      .clone()
      .unwrap()
      .memory
      .insert("move_count".to_string(), RuleExpression::Integer(1));
    board.board[0][7]
      .clone()
      .unwrap()
      .memory
      .insert("move_count".to_string(), RuleExpression::Integer(1));

    assert!(possible_moves.len() == 5, "King should have 5 moves");
    assert!(
      !possible_moves.contains(&(0u32, 6u32)),
      "king should not castle"
    );
  }

  #[test]
  fn test_long_castling() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
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

    assert!(
      !possible_moves.is_empty(),
      "king should have possible moves"
    );
    assert!(possible_moves.len() == 6, "King should have 6 moves");
    assert!(possible_moves.contains(&(7u32, 2u32)), "king should castle");
    let res = board.make_move(
      (7u32, 4u32),
      (7u32, 2u32),
      king.possiblemoves[2].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(res.is_ok(), "castling should succeed");
    let board_str = format!("{}", board);
    let board_lines = board_str
      .lines()
      .map(|line| line.to_string())
      .collect::<Vec<String>>();
    println!("{:?}", board_lines);
    assert!(board_lines[7] == ". . k r . . . .", "board after castle");

    board.board[7][3] = None;

    board.current_player = Player::Black;
    //assert if black can also castle
    let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
    assert!(
      !possible_moves.is_empty(),
      "king should have possible moves"
    );
    println!("{:?}", possible_moves);

    assert!(possible_moves.len() == 6, "King should have 6 moves");
    assert!(possible_moves.contains(&(0u32, 2u32)), "king should castle");

    //assert that blocking prevents castling
    rook.set_owner(Player::White);
    board.board[0][1] = Some(rook.clone());

    let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
    assert!(
      possible_moves.len() == 5,
      "King should have 5 moves, blocked path"
    );
    assert!(
      !possible_moves.contains(&(0u32, 2u32)),
      "king should not castle"
    );

    board.board[0][1] = None;
    board.board[1][2] = Some(rook.clone());

    let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
    assert!(
      possible_moves.len() == 5,
      "King should have 5 moves, field attacked1"
    );
    assert!(
      !possible_moves.contains(&(0u32, 2u32)),
      "king should not castle"
    );

    board.board[1][2] = None;
    board.board[1][3] = Some(rook.clone());

    let possible_moves = board.get_possible_positions((0, 4), 0).unwrap();
    assert!(
      possible_moves.len() == 5,
      "King should have 5 moves, field attacked2"
    );
    assert!(
      !possible_moves.contains(&(0u32, 2u32)),
      "king should not castle"
    );
    board.board[1][3] = None;

    //assert move_count prevets castling
    board.board[0][0]
      .clone()
      .unwrap()
      .memory
      .insert("move_count".to_string(), RuleExpression::Integer(1));

    assert!(
      possible_moves.len() == 5,
      "King should have 5 moves reason mc1"
    );
    assert!(
      !possible_moves.contains(&(0u32, 2u32)),
      "king should not castle"
    );

    board.board[0][0]
      .clone()
      .unwrap()
      .memory
      .insert("move_count".to_string(), RuleExpression::Integer(0));
    board.board[0][4]
      .clone()
      .unwrap()
      .memory
      .insert("move_count".to_string(), RuleExpression::Integer(1));
    board.board[0][0]
      .clone()
      .unwrap()
      .memory
      .insert("move_count".to_string(), RuleExpression::Integer(1));

    assert!(
      possible_moves.len() == 5,
      "King should have 5 moves reason mc2"
    );
    assert!(
      !possible_moves.contains(&(0u32, 2u32)),
      "king should not castle"
    );
  }

  #[test]
  fn test_history() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();

    assert!(board.history.is_empty(), "history should have no entries");
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let _ = board.add_piece(pawn_json.clone());
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn);
    let x = board.make_move(
      (6, 6),
      (4, 6),
      board.board[6][6].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok());
    assert!(board.history.len() == 1, "history should have 1 entry");
    assert!(board.history[0].1 .1 == (4, 6), "wrong destination");
    assert!(board.history[0].1 .0 == (6, 6), "wrong source");
    assert!(
      board.history[0].0 == board.board[4][6].as_ref().unwrap().possiblemoves[0].clone(),
      "wrong move"
    );
  }

  #[test]
  fn test_depth_err() {
    let mut board = Board::new();
    let mut context = HashMap::new();
    let res =
      RuleExpression::Boolean(false).evaluate(&mut board, &mut context, 300, None, None, false);
    assert!(res == RuleExpression::Err("too many calls".to_string()))
  }

  #[test]
  fn test_revive() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    board.revive = RuleExpression::Boolean(true);
    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let _ = board.add_piece(pawn_json.clone());
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");

    board.cementaries.0.push(pawn.clone());
    let res = board.revive_piece(true, 0, (1, 2), &tx, &rx2, false);
    assert!(res.is_ok(), "revive should succeed");
    assert!(board.board[1][2].is_some(), "there should be piece");
    assert!(board.board[1][2].as_ref().unwrap().id == "pawn_1".to_string());
  }

  #[test]
  fn test_is_same_line() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let _ = board.add_piece(pawn_json.clone());
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[6][6] = Some(pawn.clone());
    pawn.owner = Player::Black;
    board.board[6][7] = Some(pawn);
    let king_json = std::fs::read_to_string("res/chess/king.json").ok().unwrap();
    let king: Piece = Piece::new(king_json.to_string()).expect("Failed to create pawn");
    board.board[7][7] = Some(king.clone());

    let mut context = HashMap::new();
    let res = board.call_api(
      "is_same_line",
      vec!["6,6", "7,7", "owner"],
      &mut context,
      0,
      Some(&tx),
      Some(&rx2),
      false,
    );
    assert!(res == RuleExpression::Boolean(true), "same owner");
    let res = board.call_api(
      "is_same_line",
      vec!["6,6", "7,7", "piece"],
      &mut context,
      0,
      Some(&tx),
      Some(&rx2),
      false,
    );
    assert!(res == RuleExpression::Boolean(false), "different piece");
    let res = board.call_api(
      "is_same_line",
      vec!["6,6", "6,7", "piece"],
      &mut context,
      0,
      Some(&tx),
      Some(&rx2),
      false,
    );
    assert!(res == RuleExpression::Boolean(true), "same piece");
    let res = board.call_api(
      "is_same_line",
      vec!["6,6", "6,7", "owner"],
      &mut context,
      0,
      Some(&tx),
      Some(&rx2),
      false,
    );
    assert!(res == RuleExpression::Boolean(false), "different owner");
  }

  /*
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
  }*/

  #[test]
  fn test_create() {
    let x_json = std::fs::read_to_string("res/tictactoe/x.json")
      .ok()
      .unwrap();
    let x: Piece = Piece::create(x_json.to_string(), -1).expect("Failed to create x");
    let x1: Piece = Piece::create(x_json.to_string(), 0).expect("Failed to create x");
    let x2: Piece = Piece::create(x_json.to_string(), 4218).expect("Failed to create x");

    assert!(x.memory.get("id") == Some(&RuleExpression::Integer(-1)));
    assert!(x1.memory.get("id") == Some(&RuleExpression::Integer(0)));
    assert!(x2.memory.get("id") == Some(&RuleExpression::Integer(4218)));
  }

  #[test]
  fn test_west_apicall() {
    let mut board = Board::new();
    let mut context = HashMap::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[3][2] = Some(pawn);
    context.insert(
      String::from("old_position"),
      format!("{},{}", 3, 2).to_string(),
    );
    println!(
      "{:?}",
      board.call_api(
        "west",
        vec!["position", "5,6"],
        &mut context,
        0,
        None,
        None,
        false
      )
    );
    assert!(
      board.call_api(
        "west",
        vec!["position", "5,6"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Integer(-4),
      "-4 to the west"
    );
  }

  #[test]
  fn test_north_apicall() {
    let mut board = Board::new();
    let mut context = HashMap::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[3][2] = Some(pawn);
    context.insert(
      String::from("old_position"),
      format!("{},{}", 3, 2).to_string(),
    );
    println!(
      "{:?}",
      board.call_api(
        "north",
        vec!["position", "5,6"],
        &mut context,
        0,
        None,
        None,
        false
      )
    );
    assert!(
      board.call_api(
        "north",
        vec!["position", "5,6"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Integer(-2),
      "-2 to the north"
    );
  }

  #[test]
  fn test_forward_apicall() {
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let _ = board.add_piece(pawn_json.clone());
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[3][2] = Some(pawn.clone());
    pawn.owner = Player::Black;
    let mut context = HashMap::new();
    context.insert(
      String::from("old_position"),
      format!("{},{}", 3, 2).to_string(),
    );
    assert!(
      board.call_api(
        "forward",
        vec!["position", "5,6"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Integer(-2),
      "-2 forward"
    );
    board.board[3][2] = Some(pawn);
    assert!(
      board.call_api(
        "forward",
        vec!["position", "5,6"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Integer(2),
      "2 forward"
    );
  }

  #[test]
  fn test_left_apicall() {
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/chess/pawn.json").ok().unwrap();
    let _ = board.add_piece(pawn_json.clone());
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    board.board[3][2] = Some(pawn.clone());
    pawn.owner = Player::Black;
    let mut context = HashMap::new();
    context.insert(
      String::from("old_position"),
      format!("{},{}", 3, 2).to_string(),
    );
    assert!(
      board.call_api(
        "left",
        vec!["position", "5,6"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Integer(-4),
      "-4 forward"
    );
    board.board[3][2] = Some(pawn);
    assert!(
      board.call_api(
        "left",
        vec!["position", "5,6"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Integer(4),
      "4 forward"
    );
  }

  #[test]
  fn test_get_position_from_relative() {
    let board = Board::new();
    assert!(board
      .get_position_from_relative("3,2", (1, 4))
      .is_some_and(|x| x == (4, 6)));
  }

  #[test]
  fn test_get_position_from_pawn_relative() {
    let board = Board::new();
    assert!(board
      .get_position_from_pawn_relative("3,2", (1, 4), true)
      .is_some_and(|x| x == (4, 6)));
    assert!(board
      .get_position_from_pawn_relative("3,2", (1, 4), false)
      .is_none());
    assert!(board
      .get_position_from_pawn_relative("3,2", (2, 1), false)
      .is_some_and(|x| x == (1, 1)));
  }

  #[test]
  fn test_first_second() {
    let mut board = Board::new();
    let mut context = HashMap::new();
    assert!(
      RuleExpression::First(Box::new(RuleExpression::Pair(
        Box::new(RuleExpression::Boolean(true)),
        Box::new(RuleExpression::Boolean(false))
      )))
      .evaluate(&mut board, &mut context, 0, None, None, false)
        == RuleExpression::Boolean(true)
    );
  }

  #[test]
  fn test_checkmate() {
    let mut board = Board::new();

    let rook_json = std::fs::read_to_string("res/chess/rook.json").ok().unwrap();
    let mut rook: Piece = Piece::create(rook_json.to_string(), 1).expect("Failed to create rook");
    let bishop_json = std::fs::read_to_string("res/chess/bishop.json")
      .ok()
      .unwrap();
    let mut bishop: Piece =
      Piece::create(bishop_json.to_string(), 2).expect("Failed to create bishop");
    let king_json = std::fs::read_to_string("res/chess/king.json").ok().unwrap();
    let mut king: Piece = Piece::create(king_json.to_string(), 3).expect("Failed to create king");

    king.set_owner(Player::White);
    rook.set_owner(Player::Black);
    bishop.set_owner(Player::Black);

    board.board[0][0] = Some(king);
    board.board[1][1] = Some(bishop);
    board.board[0][1] = Some(rook.clone());
    board.board[1][0] = Some(rook.clone());

    let x = board.check_mate(Player::White, 3);
    println!("iks: {:?}", x);
    assert!(x == RuleExpression::Boolean(false));

    board.board[1][1] = Some(rook);

    let x = board.check_mate(Player::White, 3);
    println!("iks2: {:?}", x);
    assert!(x == RuleExpression::Boolean(true));
  }

  #[test]
  fn test_is_path_blocked() {
    let mut board = Board::new();
    let mut context = HashMap::new();

    let rook_json = std::fs::read_to_string("res/chess/rook.json").ok().unwrap();
    let rook: Piece = Piece::create(rook_json.to_string(), 1).expect("Failed to create rook");
    context.insert("old_position".to_string(), "0,0".to_string());
    context.insert("new_position".to_string(), "2,2".to_string());
    board.board[1][1] = Some(rook.clone());
    println!(
      "{:?}",
      board.call_api(
        "is_path_blocked",
        vec!["old_position", "new_position"],
        &mut context,
        0,
        None,
        None,
        false
      )
    );
    assert!(
      board.call_api(
        "is_path_blocked",
        vec!["old_position", "new_position"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(true),
      "path should be blocked"
    );
    context.insert("new_position".to_string(), "3,6".to_string());
    println!(
      "{:?}",
      board.call_api(
        "is_path_blocked",
        vec!["old_position", "new_position"],
        &mut context,
        0,
        None,
        None,
        false
      )
    );
    assert!(
      board.call_api(
        "is_path_blocked",
        vec!["old_position", "new_position"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(false),
      "path should not be blocked"
    );
  }

  #[test]
  fn test_create_var_from_mem() {
    let mut board = Board::new();
    let mut context = HashMap::new();

    let rook_json = std::fs::read_to_string("res/chess/rook.json").ok().unwrap();
    let rook: Piece = Piece::create(rook_json.to_string(), 1).expect("Failed to create rook");

    board.board[1][1] = Some(rook.clone());
    board.board[1][1]
      .as_mut()
      .unwrap()
      .memory
      .insert("mem1".to_string(), RuleExpression::Integer(1));
    board.board[1][1]
      .as_mut()
      .unwrap()
      .memory
      .insert("id".to_string(), RuleExpression::Integer(15));
    board.call_api(
      "create_var_from_mem",
      vec!["pos", "1,1", "mem1", "var1"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(
      context.contains_key("var1") && context.get("var1").unwrap() == "1",
      "there should be variables in context"
    );
    board.board[1][1].as_mut().unwrap().memory.insert(
      "mem2".to_string(),
      RuleExpression::Pair(
        Box::new(RuleExpression::Integer(2)),
        Box::new(RuleExpression::Integer(4)),
      ),
    );
    context.insert("old_position".to_string(), "1,1".to_string());
    board.call_api(
      "create_var_from_mem",
      vec!["var", "old_position", "mem2", "var2"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(
      context.contains_key("var1") && context.get("var1").unwrap() == "1",
      "there should be variables in context"
    );
    assert!(
      context.contains_key("var2") && context.get("var2").unwrap() == "2,4",
      "there should be variables in context"
    );

    board.board[1][1]
      .as_mut()
      .unwrap()
      .memory
      .insert("mem3".to_string(), RuleExpression::Boolean(false));
    board.call_api(
      "create_var_from_mem",
      vec!["id", "15", "mem3", "var3"],
      &mut context,
      0,
      None,
      None,
      false,
    );

    assert!(
      context.contains_key("var1") && context.get("var1").unwrap() == "1",
      "there should be variables in context"
    );
    assert!(
      context.contains_key("var2") && context.get("var2").unwrap() == "2,4",
      "there should be variables in context"
    );
    println!("{:?}", context);
    assert!(
      context.contains_key("var3") && context.get("var3").unwrap() == "false",
      "there should be variables in context"
    );
  }

  #[test]
  fn test_create_var_from_history() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();
    let mut context = HashMap::new();

    let rook_json = std::fs::read_to_string("res/chess/rook.json").ok().unwrap();
    let rook: Piece = Piece::create(rook_json.to_string(), 1).expect("Failed to create rook");

    board.board[1][1] = Some(rook);
    let _ = board.make_move(
      (1, 1),
      (0, 1),
      board.board[1][1].as_ref().unwrap().possiblemoves[1].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(board.board[0][1].is_some(), "move should succeed");

    board.call_api(
      "create_var_from_history",
      vec!["src", "pair", "0", "var1"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(context.contains_key("var1") && context.get("var1").unwrap() == "1,1");

    board.call_api(
      "create_var_from_history",
      vec!["dest", "v", "0", "var2"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(context.contains_key("var1") && context.get("var1").unwrap() == "1,1");
    // println!("{:?}",context);
    assert!(context.contains_key("var2") && context.get("var2").unwrap() == "0");

    board.call_api(
      "create_var_from_history",
      vec!["source", "horizontal", "0", "var3"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(context.contains_key("var1") && context.get("var1").unwrap() == "1,1");
    // println!("{:?}",context);
    assert!(context.contains_key("var2") && context.get("var2").unwrap() == "0");
    assert!(context.contains_key("var3") && context.get("var3").unwrap() == "1");
  }

  #[test]
  fn test_increase_var() {
    let mut board = Board::new();
    let mut context = HashMap::new();

    context.insert("test1".to_string(), "6,10".to_string());
    context.insert("test2".to_string(), "6".to_string());
    board.call_api(
      "increase_var",
      vec!["pair", "test1", "3,-8"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(context.get("test1").unwrap() == "9,2");
    let x = board.call_api(
      "increase_var",
      vec!["pair", "test2", "-38"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(x == RuleExpression::Err("third argument not a pair".to_string()));
    board.call_api(
      "increase_var",
      vec!["int", "test2", "-38"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(context.get("test2").unwrap() == "-32");
  }

  #[test]
  fn test_atomic_onkill1() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/atomic/atomic_pawn.json")
      .ok()
      .unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    pawn.set_owner(Player::Black);
    let bishop_json = std::fs::read_to_string("res/atomic/atomic_bishop.json")
      .ok()
      .unwrap();
    board.add_piece(bishop_json.clone()).ok();
    let bishop: Piece = Piece::new(bishop_json.to_string()).expect("Failed to create pawn");
    let king_json = std::fs::read_to_string("res/atomic/atomic_king.json")
      .ok()
      .unwrap();
    board.add_piece(king_json.clone()).ok();
    let king: Piece = Piece::new(king_json.to_string()).expect("Failed to create pawn");

    board.board[3][3] = Some(bishop.clone());
    board.board[4][4] = Some(pawn.clone());
    board.board[5][5] = Some(bishop.clone());
    board.board[5][4] = Some(bishop.clone());
    board.board[5][3] = Some(bishop.clone());
    board.board[4][3] = Some(bishop.clone());
    board.board[4][5] = Some(bishop.clone());
    board.board[3][4] = Some(bishop.clone());
    board.board[3][5] = Some(bishop.clone());

    let possible_moves = board.get_possible_positions((3, 3), 0).unwrap();
    println!("{:?}", possible_moves);
    assert!(
      !possible_moves.is_empty(),
      "Rook should have possible moves"
    );
    let x = board.make_move(
      (3, 3),
      (4, 4),
      board.board[3][3].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok(), "move should succeed");

    let board_str = format!("{}", board);
    println!("{}", board_str);

    assert!(
      board.board[3][3].is_none()
        && board.board[4][4].is_none()
        && board.board[5][5].is_none()
        && board.board[5][4].is_none()
        && board.board[5][3].is_none()
        && board.board[4][3].is_none()
        && board.board[4][5].is_none()
        && board.board[3][4].is_none()
        && board.board[3][5].is_none(),
      "those field should be empty"
    );
  }

  #[test]
  fn test_atomic_onkill2() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/atomic/atomic_pawn.json")
      .ok()
      .unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    pawn.set_owner(Player::Black);
    let bishop_json = std::fs::read_to_string("res/atomic/atomic_bishop.json")
      .ok()
      .unwrap();
    board.add_piece(bishop_json.clone()).ok();
    let bishop: Piece = Piece::new(bishop_json.to_string()).expect("Failed to create pawn");
    let king_json = std::fs::read_to_string("res/atomic/atomic_king.json")
      .ok()
      .unwrap();
    board.add_piece(king_json.clone()).ok();
    let king: Piece = Piece::new(king_json.to_string()).expect("Failed to create pawn");

    board.board[3][3] = Some(bishop.clone());
    board.board[4][4] = Some(pawn.clone());
    board.board[5][5] = Some(pawn.clone());
    board.board[5][4] = Some(pawn.clone());
    board.board[5][3] = Some(pawn.clone());
    board.board[4][3] = Some(pawn.clone());
    board.board[4][5] = Some(pawn.clone());
    board.board[3][4] = Some(pawn.clone());
    board.board[3][5] = Some(pawn.clone());

    let possible_moves = board.get_possible_positions((3, 3), 0).unwrap();
    println!("{:?}", possible_moves);
    assert!(
      !possible_moves.is_empty(),
      "Rook should have possible moves"
    );
    let x = board.make_move(
      (3, 3),
      (4, 4),
      board.board[3][3].as_ref().unwrap().possiblemoves[0].clone(),
      &tx,
      &rx2,
      true,
      false,
    );
    assert!(x.is_ok(), "move should succeed");

    let board_str = format!("{}", board);
    println!("{}", board_str);

    assert!(
      board.board[3][3].is_none()
        && board.board[4][4].is_none()
        && board.board[5][5].is_some()
        && board.board[5][4].is_some()
        && board.board[5][3].is_some()
        && board.board[4][3].is_some()
        && board.board[4][5].is_some()
        && board.board[3][4].is_some()
        && board.board[3][5].is_some(),
      "those field should be empty"
    );
  }

  #[test]
  fn test_atomic_movecondition() {
    let (tx, _): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (_, rx2): (Sender<String>, Receiver<String>) = channel();
    let mut board = Board::new();

    let pawn_json = std::fs::read_to_string("res/atomic/atomic_pawn.json")
      .ok()
      .unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let mut pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    pawn.set_owner(Player::Black);
    let bishop_json = std::fs::read_to_string("res/atomic/atomic_bishop.json")
      .ok()
      .unwrap();
    board.add_piece(bishop_json.clone()).ok();
    let bishop: Piece = Piece::new(bishop_json.to_string()).expect("Failed to create pawn");
    let king_json = std::fs::read_to_string("res/atomic/atomic_king.json")
      .ok()
      .unwrap();
    board.add_piece(king_json.clone()).ok();
    let king: Piece = Piece::new(king_json.to_string()).expect("Failed to create pawn");

    board.board[3][3] = Some(bishop.clone());
    board.board[4][4] = Some(pawn.clone());
    board.board[5][5] = Some(pawn.clone());
    board.board[5][4] = Some(pawn.clone());
    board.board[5][3] = Some(pawn.clone());
    board.board[4][3] = Some(pawn.clone());
    board.board[4][5] = Some(pawn.clone());
    board.board[3][4] = Some(king.clone());
    board.board[3][5] = Some(pawn.clone());

    let possible_moves = board.get_possible_positions((3, 3), 0).unwrap();
    println!("{:?}", possible_moves);
    assert!(
      !possible_moves.is_empty(),
      "Rook should have possible moves"
    );
    assert!(
      !possible_moves.contains(&(4, 4)),
      "this move should not be possible"
    );
  }

  #[test]
  fn test_disc() {
    let disc_json = std::fs::read_to_string("res/connect4/disc.json")
      .ok()
      .unwrap();
    assert!(Piece::new(disc_json.to_string()).is_ok());
  }

  #[test]
  fn test_is_type() {
    let mut board = Board::new();
    let pawn_json = std::fs::read_to_string("res/atomic/atomic_pawn.json")
      .ok()
      .unwrap();
    board.add_piece(pawn_json.clone()).ok();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");
    let mut context = HashMap::new();

    board.board[1][1] = Some(pawn.clone());
    board.cementaries.0.push(pawn.clone());

    context.insert(String::from("index"), format!("{}", 0).to_string());
    assert!(
      board.call_api(
        "is_type",
        vec!["pos", "1,1", "atomic_pawn"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(true)
    );
    assert!(
      board.call_api(
        "is_type",
        vec!["cementary_white", "index", "atomic_pawn"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(true)
    );
    assert!(
      board.call_api(
        "is_type",
        vec!["cementary_white", "index", "atomic_psawn"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(false)
    );
  }

  #[test]
  fn test_clone() {
    let mut board = Board::new();
    let mut context = HashMap::new();

    context.insert(
      String::from("old_position"),
      format!("{},{}", 1, 1).to_string(),
    );
    board.call_api(
      "clone",
      vec!["old_position", "eugh"],
      &mut context,
      0,
      None,
      None,
      false,
    );
    assert!(
      context.contains_key("eugh")
        && context.get("eugh").unwrap().clone() == format!("{},{}", 1, 1)
    );
  }

  #[test]
  fn test_crazyhouse() {
    let board_json = std::fs::read_to_string("res/crazyhouse/crazyhouse.json")
      .ok()
      .unwrap();
    let board = Board::from_json(board_json.to_string());
    assert!(board.is_ok());
  }

  #[test]
  fn test_is_player() {
    let mut board = Board::new();
    let mut context = HashMap::new();
    let pawn_json = std::fs::read_to_string("res/atomic/atomic_pawn.json")
      .ok()
      .unwrap();
    let pawn: Piece = Piece::new(pawn_json.to_string()).expect("Failed to create pawn");

    board.board[1][1] = Some(pawn.clone());
    context.insert(
      String::from("old_position"),
      format!("{},{}", 1, 1).to_string(),
    );
    assert!(
      board.call_api(
        "is_player",
        vec!["1,1", "White"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(true)
    );
    assert!(
      board.call_api(
        "is_player",
        vec!["old_position", "White"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(true)
    );
    assert!(
      board.call_api(
        "is_player",
        vec!["1,0", "White"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Boolean(false)
    );
    assert!(
      board.call_api(
        "is_player",
        vec!["aaa", "White"],
        &mut context,
        0,
        None,
        None,
        false
      ) == RuleExpression::Err("no aaa in is_player".to_string())
    );
  }
}
