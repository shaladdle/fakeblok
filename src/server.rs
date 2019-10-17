use crate::game;
use crate::rpc_service;
use futures::future::{self, Ready};
use futures::prelude::*;
use log::debug;
use piston_window::{Button, ButtonArgs, ButtonState, Input, Key};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::{io, net};
use tarpc::context;
use tokio::sync::watch;

fn process_input(game: &mut game::Game, input: &Input) {}

pub type PlayerId = usize;

pub struct Server {
    players: Mutex<HashMap<net::SocketAddr, PlayerId>>,
    player_ids: Mutex<slab::Slab<()>>,
    game: Arc<Mutex<game::Game>>,
    game_rx: watch::Receiver<game::Game>,
}

impl Server {
    pub fn new(game: Arc<Mutex<game::Game>>, game_rx: watch::Receiver<game::Game>) -> Self {
        let players = Mutex::new(HashMap::new());
        let player_ids = Mutex::new(slab::Slab::with_capacity(100));
        Server {
            game,
            players,
            player_ids,
            game_rx,
        }
    }

    pub fn new_handler_for_ip(&self, peer_addr: net::SocketAddr) -> io::Result<ConnectionHandler> {
        let player_id = if let Some(id) = self.get_or_create_player_id(peer_addr) {
            id
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "There are too many players connected, please try again later.",
            ));
        };
        Ok(ConnectionHandler {
            player_id: player_id,
            game: self.game.clone(),
            game_rx: self.game_rx.clone(),
        })
    }

    pub fn get_or_create_player_id(&self, peer_addr: net::SocketAddr) -> Option<PlayerId> {
        match self.players.lock().unwrap().entry(peer_addr) {
            Entry::Occupied(player_id) => Some(*player_id.get()),
            Entry::Vacant(entry) => Some(*entry.insert({
                let mut player_ids = self.player_ids.lock().unwrap();
                if player_ids.len() == player_ids.capacity() {
                    return None;
                }
                player_ids.insert(())
            })),
        }
    }
}

#[derive(Clone)]
pub struct ConnectionHandler {
    player_id: PlayerId,
    game: Arc<Mutex<game::Game>>,
    game_rx: watch::Receiver<game::Game>,
}

impl rpc_service::Game for ConnectionHandler {
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
                    let _ = game.process_key_press(&key);
                }
                ButtonState::Release => {
                    let _ = game.process_key_release(&key);
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
    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }
}
