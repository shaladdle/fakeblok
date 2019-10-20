use crate::{game::{self, EntityId, Game}, rpc_service};
use futures::future::TryFutureExt;
use futures::Future;
use futures::{channel::mpsc, stream::StreamExt};
use log::{debug, error, info};
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, EventSettings, Events, Key, Input,
    Loop, OpenGL, PistonWindow, WindowSettings,
};
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
        debug!("push_input({:?})", input);
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

const VALID_KEYS: [Key; 4] = [Key::W, Key::A, Key::S, Key::D];

fn process_input(
    game: &mut Game,
    id: EntityId,
    input: &Input,
    client: &mut GameClient,
) {
    match input {
        Input::Button(ButtonArgs {
            button: Button::Keyboard(key),
            state,
            ..
        }) if VALID_KEYS.contains(key) => match state {
            ButtonState::Press => {
                if let Ok(_) = game.process_key_press(id, key) {
                    send_keys_to_server(client, input.clone());
                }
            }
            ButtonState::Release => {
                if let Ok(_) = game.process_key_release(id, key) {
                    send_keys_to_server(client, input.clone());
                }
            }
        },
        _ => {}
    }
}

fn process_loop(game: &mut Game, lp: &Loop) {
    match lp {
        Loop::Idle(_) => {}
        Loop::Update(args) => {
            game.tick(args.dt as f32);
        }
        Loop::AfterRender(_) => {}
        lp => panic!("Didn't expect {:?}", lp),
    }
}

pub fn run_ui(server_addr: &str) -> io::Result<()> {
    let mut resolution = [512.; 2];
    let mut window: PistonWindow = WindowSettings::new("shapes", resolution)
        .exit_on_esc(true)
        .graphics_api(OpenGL::V3_2)
        .build()
        .unwrap();
    window.set_lazy(true);

    info!("Connecting to server");
    let mut client = GameClient::new(server_addr)?;
    let mut events = Events::new(EventSettings::new().ups(1000));
    info!("start!");
    let game = client.get_game();
    while let Some(event) = events.next(&mut window) {
        match event {
            Event::Input(ref input, _) => {
                process_input(&mut game.lock().unwrap(), client.id, input, &mut client);
            }
            Event::Loop(Loop::Render(args)) => {
                if resolution != args.window_size {
                    info!("Resizing {:?} => {:?}", resolution, args.window_size);
                    resolution = args.window_size;
                    window = WindowSettings::new("shapes", resolution)
                        .exit_on_esc(true)
                        .graphics_api(OpenGL::V3_2)
                        .build()
                        .unwrap();
                }
                window.draw_2d(&event, |c, g, _| {
                    clear([1.0; 4], g);
                    game.lock().unwrap().clone().draw(client.id, c, g);
                });
            }
            Event::Loop(ref lp) => {
                let mut game = game.lock().unwrap();
                process_loop(&mut game, lp);
            }
            _ => {}
        }
    }
    info!("end :(");
    Ok(())
}

fn send_keys_to_server(client: &mut GameClient, input: Input) {
    info!("send_key_to_server");
    client.push_input(input);
    info!("done");
}

