use crate::byteorder::{LittleEndian, ByteOrder, WriteBytesExt};

pub enum SyncPacket {
    ZoneID(u32),
}

pub trait EncodePacket {
    fn encode_packet(&self) -> Vec<u8>;
}

impl EncodePacket for SyncPacket {
    fn encode_packet(&self) -> Vec<u8> {
        match self {
            SyncPacket::ZoneID(zone) => encode_zone_packet(*zone)
        }
    }
}

fn encode_zone_packet(zone_id: u32) -> Vec<u8> {
    let mut packet = Vec::with_capacity(5);
    packet.push(0x01u8);
    packet.write_u32::<LittleEndian>(zone_id).unwrap();
    packet
}