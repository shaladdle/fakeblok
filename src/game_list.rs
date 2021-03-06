use futures::{
    future::{self, AbortHandle},
    prelude::*,
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map, HashMap},
    io, mem,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tarpc::{
    context,
    server::{self, Channel},
};
use tokio::time;
use tokio_serde::formats::Json;

#[derive(Debug)]
struct GameData {
    name: String,
    abort_health_check: AbortHandle,
    version: u32,
}

#[derive(Clone, Debug)]
pub struct GameList {
    peer: SocketAddr,
    games: Arc<RwLock<HashMap<SocketAddr, GameData>>>,
}

mod markers {
    pub trait Send<'a>: std::marker::Send {}
    impl<'a, T: std::marker::Send> Send<'a> for T {}
}

impl GameList {
    pub async fn run(registration_addr: SocketAddr, game_list_addr: SocketAddr) -> io::Result<()> {
        let games = Arc::new(RwLock::new(HashMap::new()));
        let (r1, r2) = future::join(
            Self::run_server(
                registration_addr,
                games.clone(),
                crate::GameRegistration::serve,
            ),
            Self::run_server(game_list_addr, games, crate::Games::serve),
        )
        .await;
        r1.and(r2)
    }

    async fn run_server<Req, Resp, Serve>(
        server_addr: SocketAddr,
        games: Arc<RwLock<HashMap<SocketAddr, GameData>>>,
        serve: impl FnMut(GameList) -> Serve + Clone,
    ) -> io::Result<()>
    where
        Serve: tarpc::server::Serve<Req, Resp = Resp> + Clone + Send + Sync + 'static,
        Req: for<'a> Deserialize<'a> + Send + 'static + Unpin,
        Resp: Serialize + Send + 'static + Unpin,
        for<'a> Serve::Fut<'a>: markers::Send<'a>,
    {
        tarpc::serde_transport::tcp::listen(&server_addr, Json::default)
            .await?
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            .map(server::BaseChannel::with_defaults)
            .map(move |channel| {
                let games = games.clone();
                let mut serve = serve.clone();
                async move {
                    let server = GameList {
                        peer: channel.get_ref().peer_addr()?,
                        games,
                    };
                    channel.execute(serve(server)).await;
                    Ok::<_, io::Error>(())
                }
            })
            .buffer_unordered(10)
            .for_each(|_| async {})
            .await;

        Ok(())
    }
}

#[tarpc::server]
impl crate::GameRegistration for GameList {
    async fn register(
        &mut self,
        _: &mut context::Context,
        port: u16,
        name: String,
    ) -> Option<String> {
        let mut game_addr = self.peer;
        game_addr.set_port(port);
        let games = self.games.clone();
        let name2 = name.clone();
        let (abort_health_check, abort_registration) = future::AbortHandle::new_pair();
        let (previous_game, version) = match self.games.write().unwrap().entry(game_addr) {
            hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().abort_health_check.abort();
                entry.get_mut().abort_health_check = abort_health_check;
                let previous_game_name = mem::replace(&mut entry.get_mut().name, name2);
                entry.get_mut().version += 1;
                (Some(previous_game_name), entry.get().version)
            }
            hash_map::Entry::Vacant(entry) => {
                entry.insert(GameData {
                    version: 0,
                    name: name2,
                    abort_health_check,
                });
                (None, 0)
            }
        };
        let health_check = future::Abortable::new(
            async move {
                struct UnregisterGame<'a> {
                    addr: SocketAddr,
                    name: &'a str,
                    games: Arc<RwLock<HashMap<SocketAddr, GameData>>>,
                    version: u32,
                }
                impl<'a> Drop for UnregisterGame<'a> {
                    fn drop(&mut self) {
                        if let hash_map::Entry::Occupied(entry) =
                            self.games.write().unwrap().entry(self.addr)
                        {
                            if entry.get().version == self.version {
                                info!(
                                    "Unregistering game {} v{}, \"{}\"",
                                    self.addr, self.version, self.name
                                );
                                entry.remove();
                            } else {
                                info!(
                                    "Game {} version is different (v{} != v{}); not unregistering",
                                    self.addr,
                                    entry.get().version,
                                    self.version
                                );
                            }
                        }
                    }
                }
                let _unregister = UnregisterGame {
                    addr: game_addr,
                    name: &name,
                    games,
                    version,
                };
                let transport =
                    match tarpc::serde_transport::tcp::connect(&game_addr, Json::default()).await {
                        Ok(transport) => transport,
                        Err(e) => {
                            warn!(
                                "Failed to connect to game {}, \"{}\": {}",
                                game_addr, name, e
                            );
                            return;
                        }
                    };
                let game_client =
                    match crate::GameClient::new(tarpc::client::Config::default(), transport)
                        .spawn()
                    {
                        Ok(game_client) => game_client,
                        Err(e) => {
                            error!(
                                "Failed to start client for game {}, \"{}\": {}",
                                game_addr, name, e
                            );
                            return;
                        }
                    };
                let mut successive_errors = 0;
                loop {
                    time::delay_for(Duration::from_secs(5)).await;
                    match game_client.ping(context::current()).await {
                        Ok(()) => successive_errors = 0,
                        Err(e) => {
                            info!("Unresponsive game {}, \"{}\": {}", game_addr, name, e);
                            if e.kind() == io::ErrorKind::ConnectionReset {
                                return;
                            }
                            successive_errors += 1;
                            if successive_errors >= 3 {
                                return;
                            }
                        }
                    }
                }
            },
            abort_registration,
        );
        tokio::spawn(health_check);
        previous_game
    }

    async fn unregister(&mut self, _: &mut context::Context, port: u16) -> Option<String> {
        let mut game_addr = self.peer;
        game_addr.set_port(port);
        self.games.write().unwrap().remove(&game_addr).map(|data| {
            data.abort_health_check.abort();
            data.name
        })
    }
}

#[tarpc::server]
impl crate::Games for GameList {
    async fn list(&mut self, _: &mut context::Context) -> HashMap<SocketAddr, String> {
        self.games
            .read()
            .unwrap()
            .iter()
            .map(|(addr, data)| (*addr, data.name.clone()))
            .collect()
    }
}
