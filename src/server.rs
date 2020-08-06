use crate::{
    game::{self, EntityId, Point},
    Game as _,
};
use futures::prelude::*;
use log::{debug, error, info};
use once_cell::sync::OnceCell;
use piston_window::{Event, EventLoop, EventSettings, Events, Loop, NoWindow, WindowSettings};
use std::{
    io,
    net::SocketAddr,
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
    peer_addr: SocketAddr,
    client_id: Arc<OnceCell<EntityId>>,
}

impl Drop for Disconnect {
    fn drop(&mut self) {
        info!("Player {} has disconnected.", self.peer_addr);
        if let Some(id) = self.client_id.get() {
            self.game.lock().unwrap().remove_entity(*id);
        }
    }
}

impl Server {
    pub fn new(game: Arc<Mutex<game::Game>>, game_rx: watch::Receiver<game::Game>) -> Self {
        Server { game, game_rx }
    }

    pub fn new_handler(&self) -> ConnectionHandler {
        ConnectionHandler {
            entity_id: Arc::new(OnceCell::new()),
            game: self.game.clone(),
            game_rx: self.game_rx.clone(),
        }
    }

    async fn run(&mut self, server_addr: SocketAddr, name: String) -> io::Result<()> {
        let listener = tarpc::serde_transport::tcp::listen(&server_addr, Json::default).await?;
        let registration =
            tarpc::serde_transport::tcp::connect("0.0.0.0:23304", Json::default()).await?;
        let registration =
            crate::GameRegistrationClient::new(tarpc::client::Config::default(), registration)
                .spawn()?;
        registration
            .register(context::current(), server_addr.port(), name)
            .await?;
        listener
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            .map(server::BaseChannel::with_defaults)
            .map(move |channel| {
                info!("Cloning server");
                let game = self.game.clone();
                let handler = self.new_handler();
                async move {
                    let peer = channel.get_ref().peer_addr()?;
                    info!("Handler for player {} created", peer);

                    // When this future is dropped, the player will be disconnected.
                    let _disconnect = Disconnect {
                        game,
                        client_id: handler.entity_id.clone(),
                        peer_addr: peer,
                    };

                    let mut handler = handler.serve();
                    let mut response_stream = channel.requests();
                    while let Some(response) = response_stream.next().await {
                        // No need to do response handling concurrently, because these futures are
                        // very short-lived.
                        response?.execute(&mut handler).await;
                    }
                    Ok::<_, io::Error>(())
                }
            })
            .buffer_unordered(10)
            .for_each(|_| async {})
            .await;

        Ok(())
    }

    pub fn run_game(server_addr: SocketAddr, name: String) -> io::Result<()> {
        let game = game::Game::new(Point::new(10_000., 500.), 50.);
        let (game_tx, game_rx) = watch::channel(game.clone());
        let game = Arc::new(Mutex::new(game));
        let mut server = Server::new(game.clone(), game_rx);

        std::thread::spawn(move || {
            info!("Starting server.");
            Runtime::new().unwrap().block_on(async move {
                match server.run(server_addr, name).await {
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
            if let Event::Loop(ref lp) = event {
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
        }
        info!("end :(");
        Ok(())
    }
}

#[derive(Clone)]
pub struct ConnectionHandler {
    entity_id: Arc<OnceCell<EntityId>>,
    game: Arc<Mutex<game::Game>>,
    game_rx: watch::Receiver<game::Game>,
}

#[tarpc::server]
impl crate::Game for ConnectionHandler {
    async fn ping(&mut self, _: &mut context::Context) {}

    async fn get_entity_id(&mut self, _: &mut context::Context) -> game::EntityId {
        self.get_or_make_entity_id()
    }

    async fn push_input(&mut self, _: &mut context::Context, input: game::Input) {
        debug!("push_input({:?})", input);
        self.game
            .lock()
            .unwrap()
            .process_input(self.get_or_make_entity_id(), input)
    }

    async fn poll_game_state(&mut self, _: &mut context::Context) -> Box<game::Game> {
        loop {
            let game = self.game_rx.recv().await.unwrap();
            if game.positions.contains(self.get_or_make_entity_id()) {
                return Box::new(game);
            }
        }
    }
}

impl ConnectionHandler {
    fn get_or_make_entity_id(&self) -> EntityId {
        *self.entity_id.get_or_init(|| {
            let mut game = self.game.lock().unwrap();
            game.insert_new_player_square()
        })
    }
}
