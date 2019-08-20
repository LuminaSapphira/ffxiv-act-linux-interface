use crate::byteorder::{LittleEndian, ByteOrder, WriteBytesExt};
use std::io::Cursor;

pub enum SyncPacket {
    ZoneID(u32),
    MobUpdate(u16, u64, Vec<u8>),
    MobNull(u16),
}

pub trait EncodePacket {
    fn encode_packet(self) -> Vec<u8>;
}

impl EncodePacket for SyncPacket {
    fn encode_packet(self) -> Vec<u8> {
        match self {
            SyncPacket::ZoneID(zone) => encode_zone_packet(zone),
            SyncPacket::MobUpdate(index, pointer, mob_data) => encode_mob_packet(index, pointer, mob_data),
            SyncPacket::MobNull(index) => encode_mob_null_packet(index),
        }
    }
}

fn encode_zone_packet(zone_id: u32) -> Vec<u8> {
    let mut packet = Vec::with_capacity(5);
    packet.push(0x01u8);
    packet.write_u32::<LittleEndian>(zone_id).unwrap();
    packet
}

fn encode_mob_null_packet(index: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(3);
    packet.push(0x03u8);
    packet.write_u16::<LittleEndian>(index).unwrap();
    packet
}

fn encode_mob_packet(index: u16, pointer: u64, mut mob_data: Vec<u8>) -> Vec<u8> {

    let mut packet = Vec::with_capacity(mob_data.len() + 19);
    packet.push(0x02);
    let mut cursor = Cursor::new(&mut packet);
    cursor.set_position(1);
    cursor.write_u16::<LittleEndian>(index);
    cursor.write_u64::<LittleEndian>(pointer);
    cursor.write_u64::<LittleEndian>(mob_data.len() as u64).unwrap();
    packet.extend(mob_data.drain(..));
    packet

}