#![deny(clippy::all)]

pub mod ruleexpression;
pub mod piece;
pub mod board;
pub mod player;
pub mod pmove;
pub mod game;
pub mod gamestate;
pub use ruleexpression::RuleExpression;
pub use piece::Piece;
pub use board::Board;
pub use pmove::PMove;
pub use gamestate::GameState;
pub use game::Game;

#[cfg(not(feature = "napi-skip"))]
use std::sync::mpsc::{channel, Receiver, Sender};

#[cfg(not(feature = "napi-skip"))]
#[macro_use]
extern crate napi_derive;

#[cfg(not(feature = "napi-skip"))]
#[napi]
pub fn sum(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(not(feature = "napi-skip"))]
#[napi]
pub struct GameWrapper {
    sender: Sender<String>,
    receiver: Receiver<Option<String>>,
    handle: Option<std::thread::JoinHandle<()>>,
    json_sender: Sender<String>
}

#[cfg(not(feature = "napi-skip"))]
#[napi]
impl GameWrapper {
    #[napi(constructor)]
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let (result_tx, result_rx) = channel();

        let (handle, json_sender) = Game::start_game(rx, result_tx);

        GameWrapper {
            sender: tx,
            receiver: result_rx,
            handle: Some(handle),
            json_sender
        }
    }

    #[napi]
    pub fn send(&self, message: String) -> napi::Result<()> {
        self.sender
        .send(message)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn send_json(&self, message: String) -> napi::Result<()> {
        self.json_sender
        .send(message)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn receive(&self) -> napi::Result<Option<String>> {
        self.receiver
        .recv()
        .map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn close(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.send("end".to_string());
            let _ = handle.join();
        }
    }
}

