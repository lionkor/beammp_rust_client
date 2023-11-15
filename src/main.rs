use std::env;
use std::env::VarError;
use std::io::Read;
use std::iter::zip;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::net::SocketAddr::V4;
use std::process::exit;
use std::str::FromStr;
use log::{debug, error, info};
use serde::Deserialize;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};

#[derive(Deserialize, Debug)]
struct AuthResponse {
    success: bool,
    message: String,
    public_key: String,
    username: String,
}

async fn send_packet(socket: &mut TcpStream, msg: &[u8]) -> anyhow::Result<()> {
    let header = u32::to_le_bytes(msg.len() as u32);

    socket.write_all(&header).await?;
    socket.write_all(msg).await?;

    Ok(())
}

async fn recv_packet(socket: &mut TcpStream) -> anyhow::Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut header: [u8; 4] = [0; 4];

    socket.read_exact(&mut header).await?;

    let size = u32::from_le_bytes(header);
    result.resize(size as usize, 0);

    socket.read_exact(result.as_mut_slice()).await?;

    Ok(result)
}

fn parse_modlist(raw_list: &String) -> anyhow::Result<Vec<(String, usize)>> {
    if raw_list == "-" { // No mods
        return Ok(Vec::new());
    }

    let list: Vec<String> = raw_list
        .split(";")
        .filter_map(|s: &str| if s.is_empty() { None } else {  Some(s.to_string()) })
        .collect();
    let mods = list[..list.len() / 2].to_vec();
    let sizes: Vec<usize> = (&list[list.len() / 2..]).iter().map(|s| s.parse::<usize>().expect(&format!("Failed to parse size as `usize`! Number: {s}"))).collect();

    Ok(mods.into_iter().zip(sizes.into_iter()).collect())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::formatted_timed_builder().filter_level(log::LevelFilter::max()).init();

    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        error!("Expected arguments <server ip> and <server port>");
        exit(1);
    }

    let addrs = (args[1].as_str(), u16::from_str(args[2].as_str())?).to_socket_addrs()?;

    let mut socket: Option<TcpStream> = None;

    for addr in addrs {
        let client = TcpSocket::new_v4()?;
        if let Ok(client_socket) = client.connect(addr).await {
            info!("Connected to {:?}", addr);
            socket = Some(client_socket);
            break;
        }
    }

    if let None = socket {
        error!("Failed to resolve or connect to '{}', port '{}'", args[1], args[2]);
        exit(1);
    }

    let mut socket = socket.unwrap();

    let C: [u8; 1] = [('C' as u8)];

    socket.write_all(&C).await?;

    send_packet(&mut socket, &format!("VC2.0").into_bytes()[..]).await?;

    let hopefully_s: Vec<u8> = recv_packet(&mut socket).await?;
    if hopefully_s.is_empty() {
        error!("Didn't got no A, nuh uh!");
        exit(1);
    }

    match hopefully_s[0] as char {
        'E' | 'K' => {
            error!("Kicked or errored.");
            exit(1);
        }
        'S' => {
            info!("Ok!");
        }
        _ => {
            error!("Expected 'A' or 'E' | 'K', got {}", hopefully_s[0]);
            exit(1);
        }
    }

    let beammp_username = env::var("BEAMMP_USER").ok();

    let beammp_password = env::var("BEAMMP_PASS").unwrap_or(String::new());

    let mut request = reqwest::Client::new()
        .post("https://auth.beammp.com/userlogin")
        .header("Content-Type", "application/json");
    if let Some(username) = beammp_username {
        request = request.body(json!({ "username": username, "password": beammp_password }).to_string());
    }
    let response = request.send().await?;

    let auth: AuthResponse = serde_json::from_str(response.text().await?.as_str())?;

    info!("Login: {:?}", auth);

    send_packet(&mut socket, auth.public_key.as_bytes()).await?;

    let pid_str = recv_packet(&mut socket).await?;

    debug!("Pid String: {:?}", pid_str);

    let pid = u8::from_str(String::from_utf8_lossy(&pid_str[1..]).as_ref())?;

    debug!("PID: {}", pid);

    send_packet(&mut socket, &format!("SR").into_bytes()[..]).await?;

    let msg = String::from_utf8(recv_packet(&mut socket).await?)?;

    let modlist = parse_modlist(&msg)?;

    for r#mod in modlist {
        send_packet(&mut socket, &[('f' as u8)]).await?;
    }

    loop {
        let msg = recv_packet(&mut socket).await?;
        debug!("Got: '{}'", String::from_utf8(msg)?);
    }

    Ok(())
}
