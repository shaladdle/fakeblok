use std::{collections::HashMap, net::SocketAddr};

pub mod client;
pub mod game;
pub mod game_list;
pub mod server;

#[tarpc::service]
pub trait Game {
    async fn ping();
    async fn get_entity_id() -> game::EntityId;
    async fn push_input(input: game::Input);
    async fn poll_game_state() -> game::Game;
}

#[tarpc::service]
pub trait GameRegistration {
    /// Registers a game associated with the client.
    /// As there can only be one registered game associated with a client,
    /// unregisters any already-registered game associated with the client.
    async fn register(port: u16, name: String) -> Option<String>;
    /// Unregisters the game associated with the client.
    /// Returns the name of the game unregistered, if any was registered.
    async fn unregister(port: u16) -> Option<String>;
}

#[tarpc::service]
pub trait Games {
    /// Lists the names of all registered games and where to find them.
    async fn list() -> HashMap<SocketAddr, String>;
}
