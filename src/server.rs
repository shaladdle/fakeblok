use crate::game::{self, EntityId};
use crate::rpc_service;
use futures::future::{self, Ready};
use futures::prelude::*;
use log::debug;
use piston_window::{Button, ButtonArgs, ButtonState, Input};
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tarpc::context;
use tokio::sync::watch;

// const RED: types::Rectangle<GameInt> = [1.0, 0.0, 0.0, 1.0];

pub struct Server {
    game: Arc<Mutex<game::Game>>,
    game_rx: watch::Receiver<game::Game>,
}

impl Server {
    pub fn new(game: Arc<Mutex<game::Game>>, game_rx: watch::Receiver<game::Game>) -> Self {
        Server { game, game_rx }
    }

    pub fn new_handler(&self, entity_id: EntityId) -> io::Result<ConnectionHandler> {
        Ok(ConnectionHandler {
            entity_id,
            game: self.game.clone(),
            game_rx: self.game_rx.clone(),
        })
    }
}

#[derive(Clone)]
pub struct ConnectionHandler {
    entity_id: EntityId,
    game: Arc<Mutex<game::Game>>,
    game_rx: watch::Receiver<game::Game>,
}

impl rpc_service::Game for ConnectionHandler {
    type GetEntityIdFut = Ready<EntityId>;

    fn get_entity_id(self, _: context::Context) -> Self::GetEntityIdFut {
        future::ready(self.entity_id)
    }

    type PushInputFut = Ready<()>;

    fn push_input(self, _: context::Context, input: Input) -> Self::PushInputFut {
        debug!("push_input({:?})", input);
        let mut game = self.game.lock().unwrap();
        match input {
            Input::Button(ButtonArgs {
                button: Button::Keyboard(key),
                state,
                ..
            }) => match state {
                ButtonState::Press => {
                    let _ = game.process_key_press(self.entity_id, &key);
                }
                ButtonState::Release => {
                    let _ = game.process_key_release(self.entity_id, &key);
                }
            },
            _ => {}
        }
        future::ready(())
    }

    type PollGameStateFut = Pin<Box<dyn Future<Output = game::Game> + Send>>;

    fn poll_game_state(mut self, _: context::Context) -> Self::PollGameStateFut {
        async move {
            debug!("poll_game_state()");
            let result = self.game_rx.recv().await.unwrap();
            debug!("poll_game_state() end");
            result
        }
            .boxed()
    }
}

impl ConnectionHandler {
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
}
