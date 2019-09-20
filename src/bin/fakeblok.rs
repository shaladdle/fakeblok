use clap::{App, Arg};
use fakeblok::game::Game;
use fakeblok::game_client;
use log::info;
use piston_window::{
    clear, Button, ButtonArgs, ButtonState, Event, EventLoop, EventSettings, Events, Input, Key,
    Loop, OpenGL, PistonWindow, WindowSettings,
};
use pretty_env_logger;
use std::collections::HashSet;
use std::io;
use tokio::runtime::Runtime;

fn process_input(keys: &mut HashSet<Key>, input: &Input) {
    match input {
        Input::Button(ButtonArgs {
            button: Button::Keyboard(key),
            state,
            ..
        }) => match state {
            ButtonState::Press => {
                keys.insert(*key);
            }
            ButtonState::Release => {
                keys.remove(key);
            }
        },
        _ => {}
    }
}

fn process_loop(game: &mut Game, lp: &Loop, keys: &HashSet<Key>) {
    match lp {
        Loop::Idle(_) => {}
        Loop::Update(_) => {
            game.tick();
            for key in keys {
                game.process_key(key);
            }
        }
        Loop::AfterRender(_) => {}
        lp => panic!("Didn't expect {:?}", lp),
    }
}

fn run_ui(server_addr: &str) -> io::Result<()> {
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("shapes", [512; 2])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();
    window.set_lazy(true);

    info!("Connecting to server");
    let mut client = game_client::GameClient::new(server_addr)?;
    let mut events = Events::new(EventSettings::new().ups(1000));
    info!("start!");
    let game = client.get_game();
    let mut keys = HashSet::new();
    while let Some(event) = events.next(&mut window) {
        match event {
            Event::Input(ref input, _) => {
                process_input(&mut keys, input);
                send_keys_to_server(&mut client, input.clone());
            }
            Event::Loop(Loop::Render(_)) => {
                window.draw_2d(&event, |c, g, _| {
                    clear([1.0; 4], g);
                    game.lock().unwrap().clone().draw(c, g);
                });
            }
            Event::Loop(ref lp) => {
                let mut game = game.lock().unwrap();
                process_loop(&mut game, lp, &keys);
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
