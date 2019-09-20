use crate::game;
use crate::rpc_service;
use futures::future::{self, Ready};
use log::debug;
use piston_window::{Button, ButtonArgs, ButtonState, Input, Key};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tarpc::context;

fn process_input(keys: &mut HashSet<Key>, input: &Input) {
    match input {
        Input::Button(ButtonArgs {
            button: Button::Keyboard(key),
            state,
            ..
        }) => match state {
            ButtonState::Press => {
                keys.insert(*key);
            }
            ButtonState::Release => {
                keys.remove(key);
            }
        },
        _ => {}
    }
}

#[derive(Clone)]
pub struct Server {
    game: Arc<Mutex<game::Game>>,
    keys: Arc<Mutex<HashSet<Key>>>,
}

impl Server {
    pub fn new(game: Arc<Mutex<game::Game>>, keys: Arc<Mutex<HashSet<Key>>>) -> Self {
        Server { game, keys }
    }
}

impl rpc_service::Game for Server {
    type PushInputFut = Ready<()>;

    fn push_input(self, _: context::Context, input: Input) -> Self::PushInputFut {
        debug!("push_input({:?})", input);
        let mut keys = self.keys.lock().unwrap();
        process_input(&mut keys, &input);
        future::ready(())
    }

    type PollGameStateFut = Ready<game::Game>;

    fn poll_game_state(self, _: context::Context) -> Self::PollGameStateFut {
        debug!("poll_game_state()");
        let result = future::ready(self.game.lock().unwrap().clone());
        debug!("poll_game_state() end");
        result
    }
}
