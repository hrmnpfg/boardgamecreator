
#[derive(Debug, Clone, PartialEq)]
pub enum GameState{
    BlackWins,
    WhiteWins,
    Draw,
    Continue,
    Error(String)
}
