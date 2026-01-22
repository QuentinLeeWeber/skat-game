use crate::lobby::Lobby;
use anyhow::Result;
use prelude::*;
use std::env;
use std::result::Result::Ok;
use tokio::net::TcpListener;
use tokio::time::{Duration, sleep};

#[cfg(test)]
mod tests;

mod game;
mod game_handle;
mod knows_skat;
mod lobby;
mod pending_game;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("starting server...");
    let port = match env::var("SERVER_PORT") {
        Ok(val) => {
            println!("server port is: {val:?}");
            val
        }
        Err(_) => {
            println!("SERVER_PORT unset, using default port 6969");
            String::from("6969")
        }
    };

    let lobby = Lobby::new().await;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    loop {
        let (stream, addr) = listener.accept().await?;

        println!("client with ip: {}, joined!", addr);
        Lobby::add_new_player(lobby.clone(), stream, addr.to_string()).await;

        sleep(Duration::from_millis(1)).await;
    }
}
