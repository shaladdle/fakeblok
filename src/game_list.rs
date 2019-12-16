use futures::{
    future::{self, Ready, AbortHandle},
    prelude::*,
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, hash_map},
    io,
    mem,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tarpc::{
    context,
    server::{self, Channel},
};
use tokio_serde::formats::Json;
use tokio::time;

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

impl GameList {
    pub async fn run(
        registration_addr: SocketAddr,
        game_list_addr: SocketAddr,
    ) -> io::Result<()> {
        let games = Arc::new(RwLock::new(HashMap::new()));
        let (r1, r2) = future::join(
            Self::run_server(registration_addr, games.clone(), crate::GameRegistration::serve),
            Self::run_server(game_list_addr, games, crate::Games::serve),
        )
        .await;
        r1.and(r2)
    }

    async fn run_server<Req, Resp, Serve>(
        server_addr: SocketAddr,
        games: Arc<RwLock<HashMap<SocketAddr, GameData>>>,
        serve: impl FnMut(GameList) -> Serve + Clone
    ) -> io::Result<()> 
    where Serve: tarpc::server::Serve<Req, Resp=Resp> + Send + 'static,
          Req: for<'a> Deserialize<'a> + Send + 'static + Unpin,
          Resp: Serialize + Send + 'static + Unpin,
          Serve::Fut: Send + 'static,
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
                    let server = GameList { peer: channel.get_ref().peer_addr()?, games };
                    channel.respond_with(serve(server)).execute().await;
                    Ok::<_, io::Error>(())
                }
            })
            .buffer_unordered(10)
            .for_each(|_| async {})
            .await;

        Ok(())
    }
}

impl crate::GameRegistration for GameList {
    type RegisterFut = Ready<Option<String>>;

    fn register(self, _: context::Context, port: u16, name: String) -> Ready<Option<String>> {
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
        let health_check = future::Abortable::new(async move {
            struct UnregisterGame<'a> {
                addr: SocketAddr,
                name: &'a str,
                games: Arc<RwLock<HashMap<SocketAddr, GameData>>>,
                version: u32,
            }
            impl<'a> Drop for UnregisterGame<'a> {
                fn drop(&mut self) {
                    if let hash_map::Entry::Occupied(entry) = self.games.write().unwrap().entry(self.addr) {
                        if entry.get().version == self.version {
                            info!("Unregistering game {} v{}, \"{}\"", self.addr, self.version, self.name);
                            entry.remove();
                        } else {
                            info!("Game {} version is different (v{} != v{}); not unregistering",
                            self.addr, entry.get().version, self.version);
                        }
                    }
                }
            }
            let _unregister = UnregisterGame {
                addr: game_addr, games: games, name: &name, version,
            };
            let transport = match tarpc::serde_transport::tcp::connect(
                &game_addr, Json::default()).await
            {
                Ok(transport) => transport,
                Err(e) => {
                    warn!("Failed to connect to game {}, \"{}\": {}", game_addr, name, e);
                    return
                }
            };
            let mut game_client = match crate::GameClient::new(
                tarpc::client::Config::default(), transport).spawn()
            {
                Ok(game_client) => game_client,
                Err(e) => {
                    error!("Failed to start client for game {}, \"{}\": {}", game_addr, name, e);
                    return
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
                            return
                        }
                    }
                }
            }
        }, abort_registration);
        tokio::spawn(health_check);
        future::ready(previous_game)
    }

    type UnregisterFut = Ready<Option<String>>;

    fn unregister(self, _: context::Context, port: u16) -> Ready<Option<String>> {
        let mut game_addr = self.peer;
        game_addr.set_port(port);
        future::ready(self.games.write().unwrap().remove(&game_addr).map(|data| {
            data.abort_health_check.abort();
            data.name
        }))
    }
}

impl crate::Games for GameList {
    type ListFut = Ready<HashMap<SocketAddr, String>>;

    fn list(self, _: context::Context) -> Ready<HashMap<SocketAddr, String>> {
        future::ready(self.games.read().unwrap().iter().map(|(addr, data)| (*addr, data.name.clone())).collect())
    }
}
