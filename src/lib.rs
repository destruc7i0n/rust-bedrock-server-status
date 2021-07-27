use std::{convert::TryInto, net::{UdpSocket, Ipv4Addr}, str, time::{Duration, SystemTime, UNIX_EPOCH}};

use rand;

#[derive(Debug)]
pub struct Server {
  pub host: String,
  pub port: i32,
  pub remote_host: String,
  pub guid: i64,
  pub edition: String,
  pub motd: [String; 2],
}

#[derive(Debug)]
pub struct Players {
  pub online: i32,
  pub max: i32
}

#[derive(Debug)]
pub struct Version {
  pub protocol: i32,
  pub name: String,
}

#[derive(Debug)]
pub struct Status {
  pub server: Server,
  pub version: Version,

  pub players: Players
}

// https://wiki.vg/Raknet_Protocol
// 00ffff00fefefefefdfdfdfd12345678
static MAGIC: [u8; 16] = [0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78];

pub fn status (h: String, p: Option<i32>) -> Result<Status, Box<dyn std::error::Error>> {
  // default port
  let port = p.unwrap_or(19132);

  let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("could not bind to local address");
  socket.connect(&format!("{}:{}", h, port)).expect("connection with server failed");

  // timeout
  socket.set_read_timeout(Some(Duration::new(2, 0)))?;
  socket.set_write_timeout(Some(Duration::new(2, 0)))?;

  let mut buf: Vec<u8> = Vec::new();
  buf.push(0x01);

  let start = SystemTime::now();
  let since_epoch: i64 = start.duration_since(UNIX_EPOCH)?.as_millis().try_into()?;
  buf.append(&mut since_epoch.to_be_bytes().to_vec());

  buf.append(&mut MAGIC.to_vec());

  // https://xkcd.com/221/
  // buf.append(&mut vec![4_u8; 8]);

  // generate random guid
  let client_guid: [u8; 8] = rand::random();
  buf.append(&mut client_guid.to_vec());

  socket.send(&buf).expect("could not send message");

  // pong
  let mut packet = [0u8; 1024];
  let (amt, src) = socket.recv_from(&mut packet).expect("could not get status");

  // get the server guid from the packet
  let guid_bytes = &packet[(8+1)..(8+8+1)];
  let guid = i64::from_be_bytes([ guid_bytes[0], guid_bytes[1], guid_bytes[2], guid_bytes[3], guid_bytes[4], guid_bytes[5], guid_bytes[6], guid_bytes[7] ]);

  // skip unused data
  let server_data_bytes = &packet[(8 + 8 + 16 + 2 + 1)..amt];
  let server_data  = str::from_utf8(&server_data_bytes).expect("could not decode server data");

  let server_data_parts = server_data.split(";").take(9).collect::<Vec<_>>();
  // println!("{:?}", server_data_parts);

  let get_part_string = |index: usize| -> String { match server_data_parts.get(index) { Some(&s) => s, None => "" }.to_string() };

  let server_unique_id = get_part_string(6);
  if server_unique_id != guid.to_string() {
    // sometimes it is different
    // guid = server_unique_id.parse::<i64>().unwrap_or(guid);
  }

  Ok(Status {
    server: Server {
      host: h,
      port,
      guid,
      remote_host: src.to_string(),
      motd: [get_part_string(1), get_part_string(7)],
      edition: get_part_string(0),
    },

    version: Version {
      protocol: get_part_string(2).parse::<i32>().unwrap_or(1),
      name: get_part_string(3),
    },
    
    players: Players {
      online: get_part_string(4).parse::<i32>().unwrap_or(-1),
      max: get_part_string(5).parse::<i32>().unwrap_or(-1),
    }
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_featured_servers () {
    let servers = vec![
      "play.inpvp.net",
      "play.lbsg.net",
      "pe.mineplex.com",
      "mco.cubecraft.net",
      "geo.hivebedrock.network",
      "play.galaxite.net",
      "play.pixelparadise.gg"
    ];

    for server in servers {
      // None defaults to the default port
      let res = status(server.to_string(), None);
      println!("{:?}", res);
      assert!(res.is_ok());
    }
  }

  #[test]
  #[should_panic(expected = "could not get status")]
  fn test_fake_server () {
    status("localhost".to_string(), None).unwrap();
  }
}