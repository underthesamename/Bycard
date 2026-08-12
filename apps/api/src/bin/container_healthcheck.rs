use std::{
    env,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use anyhow::{Context, Result, bail};

const DEFAULT_API_PORT: u16 = 8080;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

fn main() -> Result<()> {
    let port = env::var("API_PORT")
        .ok()
        .map(|value| value.parse().context("API_PORT must be a valid port"))
        .transpose()?
        .unwrap_or(DEFAULT_API_PORT);
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, REQUEST_TIMEOUT)
        .context("failed to connect to the API liveness endpoint")?;
    stream.set_read_timeout(Some(REQUEST_TIMEOUT))?;
    stream.set_write_timeout(Some(REQUEST_TIMEOUT))?;
    stream
        .write_all(b"GET /health/live HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;

    let mut status_line = [0_u8; 12];
    stream
        .read_exact(&mut status_line)
        .context("failed to read the API liveness response")?;
    if status_line != *b"HTTP/1.1 200" {
        bail!("API liveness endpoint did not return HTTP 200");
    }

    Ok(())
}
