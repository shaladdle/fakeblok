use crate::{
    game::{self, EntityId, Point},
    Game as _,
};
use futures::{
    future::{self, Ready},
    prelude::*,
};
use log::{debug, error, info};
use piston_window::{
    Event, EventLoop, EventSettings, Events, Loop, NoWindow, WindowSettings,
};
use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tarpc::{
    context,
    server::{self, Channel},
};
use tokio::{runtime::Runtime, sync::watch};
use tokio_serde::formats::Json;

const UPDATES_PER_SECOND: u64 = 200;

pub struct Server {
    game: Arc<Mutex<game::Game>>,
    game_rx: watch::Receiver<game::Game>,
}

struct Disconnect {
    game: Arc<Mutex<game::Game>>,
    client_id: EntityId,
}

impl Drop for Disconnect {
    fn drop(&mut self) {
        info!("Player {} has disconnected.", self.client_id);
        self.game.lock().unwrap().remove_entity(self.client_id);
    }
}

impl Server {
    pub fn new(game: Arc<Mutex<game::Game>>, game_rx: watch::Receiver<game::Game>) -> Self {
        Server { game, game_rx }
    }

    pub fn new_handler(&self, entity_id: EntityId) -> ConnectionHandler {
        ConnectionHandler {
            entity_id,
            game: self.game.clone(),
            game_rx: self.game_rx.clone(),
        }
    }

    async fn run(&mut self, server_addr: SocketAddr) -> io::Result<()> {
        tarpc::serde_transport::tcp::listen(&server_addr, Json::default)
            .await?
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            .map(server::BaseChannel::with_defaults)
            .map(move |channel| {
                info!("Cloning server");
                let game = self.game.clone();
                let entity_id = {
                    let mut game = game.lock().unwrap();
                    game.insert_new_player_square()
                };
                let mut handler = self.new_handler(entity_id);
                info!("Handler for player with entity id {} created", entity_id);
                async move {
                    // Wait until the game state contains the new player.
                    while !handler.game_rx.recv().await.unwrap().positions.contains(entity_id) {}

                    info!("Creating response future");
                    // When this future is dropped, the player will be disconnected.
                    let _disconnect = Disconnect {
                        game,
                        client_id: entity_id,
                    };
                    let mut response_stream = channel.respond_with(handler.serve());
                    while let Some(handler) = response_stream.next().await {
                        // No need to do response handling concurrently, because these futures are
                        // very short-lived.
                        handler?.await;
                    }
                    Ok::<_, io::Error>(())
                }
            })
            .buffer_unordered(10)
            .for_each(|_| async {})
            .await;

        Ok(())
    }

    pub fn run_game(server_addr: SocketAddr) -> io::Result<()> {
        let game = game::Game::new(Point::new(10_000., 500.), 50.);
        let (game_tx, game_rx) = watch::channel(game.clone());
        let game = Arc::new(Mutex::new(game));
        let mut server = Server::new(game.clone(), game_rx);

        std::thread::spawn(move || {
            info!("Starting server.");
            Runtime::new()
                .unwrap()
                .block_on(async move {
                    match server.run(server_addr).await {
                        Err(err) => error!("Server died: {:?}", err),
                        Ok(()) => info!("Server done."),
                    }
                });
        });

        let mut window: NoWindow = WindowSettings::new("shapes", [0; 2]).build().unwrap();

        let mut events = Events::new(EventSettings::new().ups(UPDATES_PER_SECOND).ups_reset(0));
        let mut time_in_current_bucket = 0.;
        let mut ticks_in_current_bucket = 0;
        info!("start!");

        while let Some(event) = events.next(&mut window) {
            match event {
                Event::Loop(ref lp) => {
                    let now = Instant::now();

                    let mut game = game.lock().unwrap();
                    match lp {
                        Loop::Idle(_) => {}
                        Loop::Update(args) => {
                            game.tick(
                                args.dt as f32,
                                &mut time_in_current_bucket,
                                &mut ticks_in_current_bucket,
                            );
                        }
                        lp => panic!("Didn't expect {:?}", lp),
                    }
                    let game = game.clone();
                    game_tx.broadcast(game).unwrap();

                    let elapsed = now.elapsed();
                    const TWO_MILLIS: Duration = Duration::from_millis(2);
                    if elapsed > TWO_MILLIS {
                        info!("one game loop took {:?}", elapsed);
                    }
                }
                _ => {}
            }
        }
        info!("end :(");
        Ok(())
    }
}

#[derive(Clone)]
pub struct ConnectionHandler {
    entity_id: EntityId,
    game: Arc<Mutex<game::Game>>,
    game_rx: watch::Receiver<game::Game>,
}

impl crate::Game for ConnectionHandler {
    type GetEntityIdFut = Ready<EntityId>;

    fn get_entity_id(self, _: context::Context) -> Self::GetEntityIdFut {
        future::ready(self.entity_id)
    }

    type PushInputFut = Ready<()>;

    fn push_input(self, _: context::Context, input: game::Input) -> Self::PushInputFut {
        debug!("push_input({:?})", input);
        self.game.lock().unwrap().process_input(self.entity_id, input);
        future::ready(())
    }

    type PollGameStateFut = Pin<Box<dyn Future<Output = game::Game>>>;

    fn poll_game_state(mut self, _: context::Context) -> Self::PollGameStateFut {
        const FIVE_MILLIS: Duration = Duration::from_millis(5);
        Box::pin(async move {
            let now = Instant::now();
            let result = self.game_rx.recv().await.unwrap();
            let elapsed = now.elapsed();
            if elapsed > FIVE_MILLIS {
                info!("poll_game_state() took {:?}", elapsed);
            }
            result
        })
    }
}

impl ConnectionHandler {
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
}
