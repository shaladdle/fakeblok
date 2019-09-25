use clap::{App, Arg};
use fakeblok::game::{self, Game, Point};
use fakeblok::rpc_service::Game as GameRpcTrait;
use futures::{
    future::{self},
    prelude::*,
};
use log::{error, info};
use piston_window::{
    clear, Event, EventLoop, EventSettings, Events, Key, Loop, OpenGL, PistonWindow, WindowSettings,
};
use pretty_env_logger;
use std::collections::HashSet;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tarpc::server::{self, Channel};
use tokio::runtime::current_thread;

async fn run_server(
    server_addr: SocketAddr,
    game: Arc<Mutex<Game>>,
    keys: Arc<Mutex<HashSet<Key>>>,
) -> io::Result<()> {
    let server = fakeblok::server::Server::new(game, keys);

    // tarpc_json_transport is provided by the associated crate tarpc-json-transport. It makes it easy
    // to start up a serde-powered json serialization strategy over TCP.
    tarpc_json_transport::listen(&server_addr)?
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        .map(move |channel| {
            info!("Cloning server");
            let server = server.clone();
            info!("Creating response future");
            let result = channel.respond_with(server.serve()).execute();
            info!("Done");
            result
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;
    Ok(())
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

fn run_ui(game: Arc<Mutex<game::Game>>, keys: Arc<Mutex<HashSet<Key>>>) -> io::Result<()> {
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("shapes", [512; 2])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();
    window.set_lazy(true);

    let mut events = Events::new(EventSettings::new().ups(1000));
    info!("start!");
    while let Some(event) = events.next(&mut window) {
        match event {
            Event::Loop(Loop::Render(_)) => {
                window.draw_2d(&event, |c, g, _| {
                    clear([1.0; 4], g);
                    let mut game = game.lock().unwrap().clone();
                    game.draw(c, g);
                });
            }
            Event::Loop(ref lp) => {
                let mut game = game.lock().unwrap();
                let keys = keys.lock().unwrap();
                process_loop(&mut game, lp, &keys);
            }
            _ => {}
        }
    }
    info!("end :(");
    Ok(())
}

fn main() -> io::Result<()> {
    pretty_env_logger::init();

    info!("Hello");

    let flags = App::new("Fakeblok Server")
        .version("0.1")
        .author("Tim <tikue@google.com>")
        .author("Adam <aawright@google.com>")
        .about("Run a fakeblok server that clients can connect to")
        .arg(Arg::from_usage(
            "-p --port <number> Sets the port number to listen on",
        ))
        .get_matches();

    let port = flags.value_of("port").unwrap();
    let port: u16 = port
        .parse()
        .unwrap_or_else(|e| panic!(r#"--port value "{}" invalid: {}"#, port, e));

    let game = Arc::new(Mutex::new(game::Game::new(
        Point {
            x: 10_000,
            y: 10_000,
        },
        1000,
    )));
    let keys = Arc::new(Mutex::new(HashSet::new()));
    {
        let game = game.clone();
        let keys = keys.clone();
        let server_addr: SocketAddr = ([0, 0, 0, 0u8], port).into();
        std::thread::spawn(move || {
            let mut runtime = current_thread::Runtime::new().unwrap();
            info!("Start server");
            runtime.block_on(async {
                if let Err(err) = run_server(server_addr, game, keys).await {
                    error!("Error run_server_a: {:?}", err);
                }
            });
            info!("Server done");
            runtime.run().unwrap();
        });
    }
    info!("Start ui");
    run_ui(game, keys)?;
    Ok(())
}
