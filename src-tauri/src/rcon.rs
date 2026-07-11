//! Minimal Source RCON client (enough to talk to a Palworld server).
//!
//! One TCP connection per command: connect, authenticate, send, read, drop.
//! That is wasteful in theory but the dashboard issues a command every few
//! seconds at most, so a connection pool would be complexity with no payoff.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const SERVERDATA_AUTH: i32 = 3;
const SERVERDATA_EXECCOMMAND: i32 = 2;
const AUTH_ID: i32 = 1;
const CMD_ID: i32 = 2;

fn encode(id: i32, kind: i32, body: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&id.to_le_bytes());
    payload.extend_from_slice(&kind.to_le_bytes());
    payload.extend_from_slice(body.as_bytes());
    payload.push(0); // body terminator
    payload.push(0); // empty string terminator
    let mut packet = Vec::new();
    packet.extend_from_slice(&(payload.len() as i32).to_le_bytes());
    packet.extend_from_slice(&payload);
    packet
}

/// Read one RCON packet, returning (id, body).
fn read_packet(stream: &mut TcpStream) -> Result<(i32, String), String> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = i32::from_le_bytes(len_buf);
    if !(10..=4096).contains(&len) {
        return Err(format!("bogus RCON packet length: {len}"));
    }
    let mut buf = vec![0u8; len as usize];
    stream.read_exact(&mut buf).map_err(|e| e.to_string())?;
    let id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    // buf[4..8] is the type; body is the rest minus the two trailing nulls.
    let body_end = buf.len().saturating_sub(2);
    let body = String::from_utf8_lossy(&buf[8..body_end]).to_string();
    Ok((id, body))
}

/// Authenticate and run a single command, returning the server's response body.
pub fn command(host: &str, port: u16, password: &str, cmd: &str) -> Result<String, String> {
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or("could not resolve RCON address")?;

    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(4))
        .map_err(|e| format!("RCON connect failed: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    // Auth handshake. A response id of -1 means the password was rejected.
    stream
        .write_all(&encode(AUTH_ID, SERVERDATA_AUTH, password))
        .map_err(|e| e.to_string())?;
    let (auth_id, _) = read_packet(&mut stream)?;
    if auth_id == -1 {
        return Err("RCON authentication failed (wrong admin password?)".into());
    }

    stream
        .write_all(&encode(CMD_ID, SERVERDATA_EXECCOMMAND, cmd))
        .map_err(|e| e.to_string())?;
    let (_, body) = read_packet(&mut stream)?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_roundtrips_length_prefix() {
        let p = encode(CMD_ID, SERVERDATA_EXECCOMMAND, "ShowPlayers");
        let len = i32::from_le_bytes([p[0], p[1], p[2], p[3]]);
        assert_eq!(len as usize, p.len() - 4);
    }
}
