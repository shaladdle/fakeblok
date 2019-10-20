use clap::{App, Arg};
use fakeblok::game_client;
use pretty_env_logger;
use std::io;
use tokio::runtime::Runtime;

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
    let runtime = Runtime::new()?;
    tokio_executor::with_default(&mut runtime.executor(), || game_client::run_ui(server_addr))?;
    Ok(())
}
