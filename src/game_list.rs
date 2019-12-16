use futures::{
    future::{self, Ready},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tarpc::{
    context,
    server::{self, Channel},
};
use tokio_serde::formats::Json;

#[derive(Clone, Debug)]
pub struct GameList {
    peer: SocketAddr,
    games: Arc<RwLock<HashMap<SocketAddr, String>>>,
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
        games: Arc<RwLock<HashMap<SocketAddr, String>>>,
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
        future::ready(self.games.write().unwrap().insert(game_addr, name))
    }

    type UnregisterFut = Ready<Option<String>>;

    fn unregister(self, _: context::Context, port: u16) -> Ready<Option<String>> {
        let mut game_addr = self.peer;
        game_addr.set_port(port);
        future::ready(self.games.write().unwrap().remove(&game_addr))
    }
}

impl crate::Games for GameList {
    type ListFut = Ready<HashMap<SocketAddr, String>>;

    fn list(self, _: context::Context) -> Ready<HashMap<SocketAddr, String>> {
        future::ready(self.games.read().unwrap().clone())
    }
}
