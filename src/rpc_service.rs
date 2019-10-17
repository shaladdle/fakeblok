use crate::game;
use piston_window::Input;

#[tarpc::service]
pub trait Game {
    async fn get_entity_id() -> game::EntityId;
    async fn push_input(input: Input);
    async fn poll_game_state() -> game::Game;
}
