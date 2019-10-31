use piston_window::Input;

pub mod client;
pub mod game;
pub mod server;

#[tarpc::service]
pub trait Game {
    async fn get_entity_id() -> game::EntityId;
    async fn push_input(input: Input);
    async fn poll_game_state() -> game::Game;
}
