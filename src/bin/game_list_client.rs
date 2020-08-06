use clap::{App, Arg};
use log::info;
use std::{io, net::SocketAddr};
use tokio_serde::formats::Json;

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

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            let client = create_client(server_addr).await.unwrap();
            println!(
                "Available games: {:?}",
                client.list(tarpc::context::current()).await.unwrap()
            );
        });
    Ok(())
}

async fn create_client(server_addr: SocketAddr) -> io::Result<fakeblok::GamesClient> {
    info!("Creating client to {}", server_addr);
    let transport = tarpc::serde_transport::tcp::connect(&server_addr, Json::default()).await?;
    fakeblok::GamesClient::new(tarpc::client::Config::default(), transport).spawn()
}
