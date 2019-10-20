use clap::{App, Arg};
use fakeblok::server::Server;
use log::info;
use pretty_env_logger;
use std::{io, net::SocketAddr};

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
    let server_addr: SocketAddr = ([0, 0, 0, 0u8], port).into();

    info!("Starting game.");
    Server::run_game(server_addr)?;
    Ok(())
}
