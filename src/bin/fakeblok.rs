use clap::{App, Arg};
use fakeblok::client;
use pretty_env_logger;
use std::{io, net::SocketAddr};

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

    let server_addr = flags.value_of("server_addr").unwrap();
    let server_addr: SocketAddr = server_addr
        .parse()
        .unwrap_or_else(|e| panic!(r#"--server_addr value "{}" invalid: {}"#, server_addr, e));
    client::run_ui(server_addr)?;
    Ok(())
}
