use async_tungstenite::tungstenite::Message;
use bincode::Options;
use miniz_oxide::inflate::decompress_to_vec_zlib;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum OperationCode {
    Unknown = 0,
    Heartbeat = 2,
    HeartbeatResponse = 3,
    Normal = 5,
    Auth = 7,
    AuthResponse = 8,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct PacketHeader {
    total_size: u32,
    header_size: u16,
    protocol_version: u16,
    operation_code: u32,
    sequence: u32,
}

const PACKET_HEADER_SIZE: usize = std::mem::size_of::<PacketHeader>();

impl PacketHeader {
    fn new(payload_size: usize, operation_code: OperationCode) -> Self {
        PacketHeader {
            total_size: payload_size as u32 + PACKET_HEADER_SIZE as u32,
            header_size: PACKET_HEADER_SIZE as u16,
            protocol_version: 0,
            operation_code: operation_code as u32,
            sequence: 0,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        bincode::DefaultOptions::new()
            .with_big_endian()
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .deserialize(&bytes[..PACKET_HEADER_SIZE])
            .unwrap()
    }

    fn operation_code(&self) -> OperationCode {
        match self.operation_code {
            2 => OperationCode::Heartbeat,
            3 => OperationCode::HeartbeatResponse,
            5 => OperationCode::Normal,
            7 => OperationCode::Auth,
            8 => OperationCode::AuthResponse,
            _ => OperationCode::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Packet {
    header: PacketHeader,
    json_payload: Vec<u8>,
}

impl Packet {
    pub fn new_auth_packet(uid: u64, real_room_id: u64) -> Self {
        let json_payload = json!({
            "uid": uid,
            "roomid": real_room_id,
        })
        .to_string()
        .into_bytes();
        Self {
            header: PacketHeader::new(json_payload.len(), OperationCode::Auth),
            json_payload,
        }
    }

    pub fn new_heartbeat_packet() -> Self {
        Self {
            header: PacketHeader::new(0, OperationCode::Heartbeat),
            json_payload: Vec::new(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let header = PacketHeader::from_bytes(&bytes[..PACKET_HEADER_SIZE]);
        Self {
            header,
            json_payload: bytes[PACKET_HEADER_SIZE..header.total_size as usize].to_vec(),
        }
    }

    #[inline]
    pub fn get_operation_code(&self) -> OperationCode {
        self.header.operation_code()
    }

    pub fn get_command_json(&self) -> Option<Vec<String>> {
        if self.header.operation_code() != OperationCode::Normal {
            return None;
        }
        let mut commands = Vec::new();
        // If the `protocol_version` is 2, it means there are multiply commands in the payload.
        if self.header.protocol_version == 2 {
            let decompressed = decompress_to_vec_zlib(&self.json_payload).unwrap();
            let mut offset = 0;
            while offset < decompressed.len() {
                let header_offset = offset + PACKET_HEADER_SIZE;
                let header = PacketHeader::from_bytes(&decompressed[offset..header_offset]);
                commands.push(
                    String::from_utf8(
                        decompressed[header_offset..offset + header.total_size as usize].to_vec(),
                    )
                    .unwrap(),
                );
                offset += header.total_size as usize;
            }
        } else {
            commands.push(String::from_utf8(self.json_payload.clone()).unwrap())
        }
        Some(commands)
    }
}

impl From<Packet> for Message {
    fn from(mut packet: Packet) -> Self {
        let mut payload: Vec<u8> = Vec::with_capacity(packet.header.total_size as usize);
        payload.append(
            &mut bincode::DefaultOptions::new()
                .with_big_endian()
                .with_fixint_encoding()
                .serialize(&packet.header)
                .unwrap(),
        );
        payload.append(&mut packet.json_payload);
        Message::Binary(payload)
    }
}
