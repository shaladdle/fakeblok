use clap::{App, Arg};
use fakeblok::game::{EntityId, Game};
use fakeblok::game_client;
use log::info;
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, EventSettings, Events, Input, Loop,
    OpenGL, PistonWindow, WindowSettings,
};
use pretty_env_logger;
use std::io;
use tokio::runtime::Runtime;

fn process_input(
    game: &mut Game,
    id: EntityId,
    input: &Input,
    client: &mut game_client::GameClient,
) {
    match input {
        Input::Button(ButtonArgs {
            button: Button::Keyboard(key),
            state,
            ..
        }) => match state {
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

fn run_ui(server_addr: &str) -> io::Result<()> {
    let mut resolution = [512.; 2];
    let mut window: PistonWindow = WindowSettings::new("shapes", resolution)
        .exit_on_esc(true)
        .graphics_api(OpenGL::V3_2)
        .build()
        .unwrap();
    window.set_lazy(true);

    info!("Connecting to server");
    let mut client = game_client::GameClient::new(server_addr)?;
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

fn send_keys_to_server(client: &mut game_client::GameClient, input: Input) {
    info!("send_key_to_server");
    client.push_input(input);
    info!("done");
}

fn main() -> io::Result<()> {
    pretty_env_logger::init();
    let flags = App::new("Fakeblok")
        .version("0.1")
        .author("Tim <tikue@google.com>")
        .author("Adam <aawright@google.com>")
        .about("Say hello!")
        .arg(Arg::from_usage(
            "--server_addr <address> Sets the server address to connect to.",
        ))
        .get_matches();

    let runtime = Runtime::new()?;
    let server_addr = flags.value_of("server_addr").unwrap();
    tokio_executor::with_default(&mut runtime.executor(), || run_ui(server_addr))?;
    Ok(())
}
