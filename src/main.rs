use std::sync::mpsc;
use std::io::{self, Write};
use std::error::Error;

use boardgames::game::Game;

fn main() -> Result<(), Box<dyn Error>> {
    let (tx_main, rx_game) = mpsc::channel();
    let (tx_game, rx_main) = mpsc::channel();

    let (game_thread, _) = Game::start_game(rx_game, tx_game);

    println!("[Client] Board Game Interface");
    println!("[Client] -------------------");
    println!("[Client] Type your commands below. Available commands:");
    print_help();
    println!("[Client] Type anything to begin.");

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        match input.to_lowercase().as_str() {
            "exit" | "quit" => {
                println!("Exiting game...");
                break;
            }
            "help" => {
                print_help();
                continue;
            }
            "" => continue,
            _ => {
                tx_main.send(input.clone())?;
            }
        }

        while let Ok(response) = rx_main.recv() {
            match response {
                Some(res) => println!("[Game] {}", res),
                None => break,
            }
        }
    }

    drop(tx_main);

    game_thread.join().expect("[Client] Game thread panicked");

    Ok(())
}

fn print_help() {
    println!("[Client] Available commands:");
    println!("[Client] - 'help': Show this help menu");
    println!("[Client] - 'exit' or 'quit': Exit the game");
    println!("[Client] - Other commands depend on the current game state");
}
