use crate::{
    game::{self, EntityId},
};
use futures::{prelude::*, channel::mpsc};
use log::{debug, error, info};
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, EventSettings, Events, Input, Key,
    Loop, OpenGL, PistonWindow, WindowSettings,
};
use std::{io, net::SocketAddr, sync::{Arc, Condvar, Mutex}, thread};
use tarpc::client::{self, NewClient};
use tarpc::context;
use tokio::runtime::current_thread;

/// A task that pushes player inputs to the server.
struct InputPusher {
    client: crate::GameClient,
    inputs: mpsc::UnboundedReceiver<Input>,
}

impl InputPusher {
    async fn run(mut self) {
        while let Some(input) = self.inputs.next().await {
            debug!("push_input({:?})", input);
            if let Err(err) = self.client.push_input(context::current(), input.clone()).await {
                error!("Error setting keys, {:?}: {:?}", input, err);
            }
        }
    }
}

/// A task that repeatedly polls game state and updates the main thread's game state.
/// Also is responsible for waking the main thread after the first poll.
struct StatePoller {
    client: crate::GameClient,
    game: Arc<Mutex<game::Game>>,
    client_id: Arc<(Mutex<Option<EntityId>>, Condvar)>,
}

impl StatePoller {
    async fn run(mut self) {
        let mut client1 = self.client.clone();
        let mut client2 = self.client.clone();
        let game_state = client1.poll_game_state(context::current());
        let client_id = client2.get_entity_id(context::current());

        info!("Getting initial game state:");
        match future::join(game_state, client_id).await {
            (Ok(game_state), Ok(client_id)) => {
                // First poll notifies the main thread.
                *self.game.lock().unwrap() = game_state;

                // Let the main thread know we've started.
                let (lock, cvar) = &*self.client_id;
                *lock.lock().unwrap() = Some(client_id);
                cvar.notify_one();
            }
            (Err(e), _) | (_, Err(e)) => {
                error!("Could not initialize client: {}", e);
                return;
            }
        }

        loop {
            match self.client.poll_game_state(context::current()).await {
                Ok(new_game) => *self.game.lock().unwrap() = new_game,
                Err(e) => {
                    error!("Failed to poll game state: {}", e);
                    break;
                }
            }
        }
    }
}

async fn create_client(
    server_addr: SocketAddr,
) -> io::Result<(crate::GameClient, impl Future<Output = ()>)> {
    info!("Creating client to {}", server_addr);

    let transport = tarpc_json_transport::connect(&server_addr).await?;
    let NewClient { client, dispatch } =
        crate::GameClient::new(client::Config::default(), transport);
    let dispatch = dispatch.unwrap_or_else(move |e| error!("Connection broken: {}", e));
    Ok((client, dispatch))
}

async fn spawn_tasks(
    server_addr: SocketAddr,
    game: Arc<Mutex<game::Game>>,
    client_id: Arc<(Mutex<Option<EntityId>>, Condvar)>,
    inputs: mpsc::UnboundedReceiver<Input>
) -> io::Result<()> {
    let (client, dispatch) = create_client(server_addr).await?;
    tokio::spawn(dispatch);
    tokio::spawn(StatePoller { client: client.clone(), client_id, game: game.clone() }.run());
    tokio::spawn(InputPusher { client, inputs }.run());
    Ok(())
}

const VALID_KEYS: [Key; 4] = [Key::W, Key::A, Key::S, Key::D];

pub fn run_ui(server_addr: SocketAddr) -> io::Result<()> {
    let mut resolution = [512.; 2];
    let mut window: PistonWindow = WindowSettings::new("shapes", resolution)
        .exit_on_esc(true)
        .graphics_api(OpenGL::V3_2)
        .build()
        .unwrap();
    window.set_lazy(true);

    info!("Connecting to server");
    let game = Arc::new(Mutex::new(game::Game::default()));
    let client_id = Arc::new((Mutex::new(None), Condvar::new()));
    let (inputs, rx) = mpsc::unbounded();

    let game2 = game.clone();
    let client_id2 = client_id.clone();

    thread::spawn(move || {
        let mut runtime = current_thread::Runtime::new().unwrap();
        runtime.spawn(async move {
            if let Err(e) = spawn_tasks(server_addr, game2, client_id2, rx).await {
                error!("Failed to start all client tasks: {}", e);
            };
        });
        runtime.run().unwrap();
    });

    // Wait for game state to be initialized.
    let (lock, cvar) = &*client_id;
    let mut client_id = lock.lock().unwrap();
    let client_id = loop {
        match *client_id {
            Some(client_id) => break client_id,
            None => client_id = cvar.wait(client_id).unwrap(),
        }
    };

    let mut events = Events::new(EventSettings::new().ups(1000));
    info!("start!");
    while let Some(event) = events.next(&mut window) {
        match event {
            Event::Input(ref input, _) => {
                match input {
                    Input::Button(ButtonArgs {
                        button: Button::Keyboard(key),
                        state,
                        ..
                    }) if VALID_KEYS.contains(key) => {
                        let mut game = game.lock().unwrap();
                        match state {
                            ButtonState::Press => {
                                if let Ok(_) = game.process_key_press(client_id, key) {
                                    inputs.unbounded_send(input.clone()).unwrap();
                                }
                            }
                            ButtonState::Release => {
                                if let Ok(_) = game.process_key_release(client_id, key) {
                                    inputs.unbounded_send(input.clone()).unwrap();
                                }
                            }
                        }
                    },
                    _ => {}
                }
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
                    game.lock().unwrap().clone().draw(client_id, c, g);
                });
            }
            Event::Loop(ref lp) => {
                let mut game = game.lock().unwrap();
                match lp {
                    Loop::Idle(_) => {}
                    Loop::Update(args) => {
                        game.tick(args.dt as f32);
                    }
                    Loop::AfterRender(_) => {}
                    lp => panic!("Didn't expect {:?}", lp),
                }
            }
            _ => {}
        }
    }
    info!("end :(");
    Ok(())
}
