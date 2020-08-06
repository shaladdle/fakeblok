use crate::game::{self, EntityId};
use futures::{channel::mpsc, prelude::*};
use log::{debug, error, info};
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, EventSettings, Events, Input, Key,
    Loop, OpenGL, PistonWindow, WindowSettings,
};
use std::{
    convert::TryFrom,
    io,
    net::SocketAddr,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant, SystemTime},
};
use tarpc::client::{self, NewClient};
use tarpc::context;
use tokio::runtime::Runtime;
use tokio_serde::formats::Json;

const UPDATES_PER_SECOND: u64 = 200;

/// A task that pushes player inputs to the server.
struct InputPusher {
    client: crate::GameClient,
    inputs: mpsc::UnboundedReceiver<game::Input>,
}

fn new_context() -> context::Context {
    let mut ctx = context::current();
    ctx.deadline = SystemTime::now() + Duration::from_millis(150);
    ctx
}

impl InputPusher {
    async fn run(mut self) {
        while let Some(input) = self.inputs.next().await {
            debug!("push_input({:?})", input);
            if let Err(err) = self.client.push_input(new_context(), input).await {
                error!("Error setting keys, {:?}: {:?}", input, err);
            }
        }
    }
}

/// A task that repeatedly polls game state and updates the main thread's game state.
/// Also is responsible for waking the main thread after the first poll.
struct StatePoller {
    client: crate::GameClient,
    game: Arc<Mutex<Box<game::Game>>>,
    client_id: Arc<(Mutex<Option<EntityId>>, Condvar)>,
}

impl StatePoller {
    async fn run(self) {
        let game_state = self.client.poll_game_state(context::current());
        let client_id = self.client.get_entity_id(context::current());

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
            let now = Instant::now();

            match self.client.poll_game_state(new_context()).await {
                Ok(new_game) => *self.game.lock().unwrap() = new_game,
                Err(e) => {
                    error!("Failed to poll game state: {}", e);
                    break;
                }
            }

            let elapsed = now.elapsed();
            const FIFTY_MILLIS: Duration = Duration::from_millis(50);
            if elapsed > FIFTY_MILLIS {
                info!("Polling game state took {:?}", elapsed);
            }
        }
    }
}

async fn create_client(
    server_addr: SocketAddr,
) -> io::Result<(crate::GameClient, impl Future<Output = ()>)> {
    info!("Creating client to {}", server_addr);

    let transport = tarpc::serde_transport::tcp::connect(&server_addr, Json::default()).await?;
    let NewClient { client, dispatch } =
        crate::GameClient::new(client::Config::default(), transport);
    let dispatch = dispatch.unwrap_or_else(move |e| error!("Connection broken: {}", e));
    Ok((client, dispatch))
}

async fn run_tasks(
    server_addr: SocketAddr,
    game: Arc<Mutex<Box<game::Game>>>,
    client_id: Arc<(Mutex<Option<EntityId>>, Condvar)>,
    inputs: mpsc::UnboundedReceiver<game::Input>,
) -> io::Result<()> {
    let (client, dispatch) = create_client(server_addr).await?;
    let (r1, r2, r3) = future::join3(
        tokio::spawn(dispatch),
        tokio::spawn(
            StatePoller {
                client: client.clone(),
                client_id,
                game: game.clone(),
            }
            .run(),
        ),
        tokio::spawn(InputPusher { client, inputs }.run()),
    )
    .await;
    r1.and(r2)
        .and(r3)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

impl TryFrom<(&ButtonState, &Key)> for game::Input {
    type Error = game::InvalidKeyError;

    fn try_from((state, key): (&ButtonState, &Key)) -> Result<game::Input, game::InvalidKeyError> {
        use game::{Component, Input, Sign};
        Ok(match (*state, *key) {
            (ButtonState::Press, Key::W) => Input::Move(Component::Y, Some(Sign::Negative)),
            (ButtonState::Press, Key::A) => Input::Move(Component::X, Some(Sign::Negative)),
            (ButtonState::Press, Key::S) => Input::Move(Component::Y, Some(Sign::Positive)),
            (ButtonState::Press, Key::D) => Input::Move(Component::X, Some(Sign::Positive)),
            (ButtonState::Press, Key::Space) => Input::Shoot,
            (ButtonState::Release, Key::W) => Input::Move(Component::Y, None),
            (ButtonState::Release, Key::A) => Input::Move(Component::X, None),
            (ButtonState::Release, Key::S) => Input::Move(Component::Y, None),
            (ButtonState::Release, Key::D) => Input::Move(Component::X, None),
            _ => return Err(game::InvalidKeyError),
        })
    }
}

pub fn run_ui(server_addr: SocketAddr) -> io::Result<()> {
    let mut resolution = [512.; 2];
    let mut window: PistonWindow = WindowSettings::new("shapes", resolution)
        .exit_on_esc(true)
        .graphics_api(OpenGL::V3_2)
        .build()
        .unwrap();
    window.set_lazy(true);

    info!("Connecting to server");
    let game = Arc::new(Mutex::new(Box::new(game::Game::default())));
    let client_id = Arc::new((Mutex::new(None), Condvar::new()));
    let (inputs, rx) = mpsc::unbounded();

    let game2 = game.clone();
    let client_id2 = client_id.clone();

    thread::spawn(move || {
        Runtime::new().unwrap().block_on(async move {
            if let Err(e) = run_tasks(server_addr, game2, client_id2, rx).await {
                error!("{}", e);
            };
        });
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

    let mut events = Events::new(EventSettings::new().ups(UPDATES_PER_SECOND).ups_reset(0));
    let mut time_in_current_bucket = 0.;
    let mut ticks_in_current_bucket = 0;
    info!("start!");

    while let Some(event) = events.next(&mut window) {
        match event {
            Event::Input(ref input, _) => {
                if let Input::Button(ButtonArgs {
                    button: Button::Keyboard(key),
                    state,
                    ..
                }) = input
                {
                    let mut game = game.lock().unwrap();
                    if let Ok(input) = game::Input::try_from((state, key)) {
                        game.process_input(client_id, input);
                        inputs.unbounded_send(input).unwrap();
                    }
                }
            }
            Event::Loop(Loop::Render(args)) => {
                fn fuzzy_eq(resolution: [f64; 2], window_size: [f64; 2]) -> bool {
                    fn fuzzy_eq(f1: f64, f2: f64) -> bool {
                        (f1 - f2).abs() < f64::EPSILON
                    }
                    fuzzy_eq(resolution[0], window_size[0])
                        && fuzzy_eq(resolution[1], window_size[1])
                }
                if !fuzzy_eq(resolution, args.window_size) {
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
                        game.tick(
                            args.dt as f32,
                            &mut time_in_current_bucket,
                            &mut ticks_in_current_bucket,
                        );
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
