use clap::{App, Arg};
use log::info;
use std::{env, io, net::SocketAddr};
use tokio::runtime::Runtime;

fn main() -> io::Result<()> {
    let mut logger = pretty_env_logger::formatted_timed_builder();
    if let Ok(filter) = env::var("RUST_LOG") {
        logger.parse_filters(&filter);
    }
    logger.init();

    let flags = App::new("Fakeblok Listings")
        .version("0.1")
        .author("Tim <tikue@google.com>")
        .author("Adam <aawright@google.com>")
        .about("Run a fakeblok listings server that clients can use to list running games")
        .arg(Arg::from_usage(
            "-r --registration_port <number> Sets the port number the registration server listens on",
        ))
        .arg(Arg::from_usage(
            "-l --list_port <number> Sets the port number the listings server listens on",
        ))
        .get_matches();

    let registration_port = flags.value_of("registration_port").unwrap();
    let registration_port: u16 = registration_port
        .parse()
        .unwrap_or_else(|e| panic!(r#"--r value "{}" invalid: {}"#, registration_port, e));
    let registration_addr: SocketAddr = ([0, 0, 0, 0u8], registration_port).into();

    let list_port = flags.value_of("list_port").unwrap();
    let list_port: u16 = list_port
        .parse()
        .unwrap_or_else(|e| panic!(r#"--l value "{}" invalid: {}"#, list_port, e));
    let list_addr: SocketAddr = ([0, 0, 0, 0u8], list_port).into();

    info!("Starting game list server.");
    Runtime::new().unwrap().block_on(
        fakeblok::game_list::GameList::run(registration_addr, list_addr)
    )
}
