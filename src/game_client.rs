use crate::game;
use crate::game::EntityId;
use crate::rpc_service;
use futures::future::TryFutureExt;
use futures::Future;
use futures::{channel::mpsc, stream::StreamExt};
use log::{debug, error, info};
use piston_window::Input;
use std::io;
use std::sync::{Arc, Mutex};
use tarpc::client::{self, NewClient};
use tarpc::context;
use tokio::runtime::current_thread;

pub struct GameClient {
    pub id: EntityId,
    game: Arc<Mutex<game::Game>>,
    inputs: mpsc::UnboundedSender<Input>,
}

async fn create_client(
    server_addr: &str,
) -> io::Result<(rpc_service::GameClient, impl Future<Output = ()>)> {
    let server_addr = match server_addr.parse() {
        Ok(s) => s,
        // TODO: Can we also pass the parse error as the detailed error?
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Failed to parse server addr into SocketAddr",
            ))
        }
    };
    let transport = tarpc_json_transport::connect(&server_addr).await?;
    let NewClient { client, dispatch } =
        rpc_service::GameClient::new(client::Config::default(), transport);
    info!("Spawn dispatch");
    let dispatch = dispatch.unwrap_or_else(move |e| error!("Connection broken: {}", e));
    info!("Dispatch spawned");
    Ok((client, dispatch))
}

async fn push_inputs(
    mut client: rpc_service::GameClient,
    mut inputs: mpsc::UnboundedReceiver<Input>,
) {
    while let Some(input) = inputs.next().await {
        info!("push_input({:?})", input);
        if let Err(err) = client.push_input(context::current(), input.clone()).await {
            error!("Error setting keys, {:?}: {:?}", input, err);
        }
    }
}

async fn repeated_poll_game_state(
    mut client: rpc_service::GameClient,
    game: Arc<Mutex<game::Game>>,
) {
    while let Ok(new_game) = client.poll_game_state(context::current()).await {
        *game.lock().unwrap() = new_game;
    }
}

impl GameClient {
    pub fn new(server_addr: &str) -> io::Result<GameClient> {
        debug!("Creating runtime");
        let mut runtime = current_thread::Runtime::new().unwrap();
        debug!("Creating client to {}", server_addr);
        let (mut client, dispatch) = runtime.block_on(create_client(server_addr))?;
        tokio::spawn(dispatch);
        debug!("Getting entity id");
        let id = runtime.block_on(client.get_entity_id(context::current()))?;
        debug!("Getting initial game state:");
        let game = runtime.block_on(client.poll_game_state(context::current()))?;
        let game = Arc::new(Mutex::new(game));
        debug!("Successfully created new GameClient");
        let (inputs, rx) = mpsc::unbounded();
        tokio::spawn(repeated_poll_game_state(client.clone(), game.clone()));
        tokio::spawn(push_inputs(client, rx));
        Ok(GameClient { id, game, inputs })
    }

    pub fn push_input(&mut self, input: Input) {
        self.inputs.unbounded_send(input).unwrap();
    }

    pub fn get_game(&mut self) -> Arc<Mutex<game::Game>> {
        self.game.clone()
    }
}
