use crate::game;
use crate::rpc_service;
use futures::future::{self, Ready};
use log::debug;
use piston_window::{Button, ButtonArgs, ButtonState, Input};
use std::sync::{Arc, Mutex};
use tarpc::context;

fn process_input(game: &mut game::Game, input: &Input) {
    match input {
        Input::Button(ButtonArgs {
            button: Button::Keyboard(key),
            state,
            ..
        }) => match state {
            ButtonState::Press => {
                let _ = game.process_key_press(key);
            }
            ButtonState::Release => {
                let _ = game.process_key_release(key);
            }
        },
        _ => {}
    }
}

#[derive(Clone)]
pub struct Server {
    game: Arc<Mutex<game::Game>>,
}

impl Server {
    pub fn new(game: Arc<Mutex<game::Game>>) -> Self {
        Server { game }
    }
}

impl rpc_service::Game for Server {
    type PushInputFut = Ready<()>;

    fn push_input(self, _: context::Context, input: Input) -> Self::PushInputFut {
        debug!("push_input({:?})", input);
        let mut game = self.game.lock().unwrap();
        process_input(&mut game, &input);
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
