use crate::byteorder::{LittleEndian as LE, WriteBytesExt};
use std::io::Cursor;
use crate::mem::models::Target;

pub enum SyncPacket {
    ZoneID(u32),
    MobUpdate(u16, u64, Vec<u8>),
    MobNull(u16),
    Target(Target)
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
            SyncPacket::Target(target) => encode_target_packet(target),
        }
    }
}

fn encode_zone_packet(zone_id: u32) -> Vec<u8> {
    let mut packet = Vec::with_capacity(5);
    packet.push(0x01u8);
    packet.write_u32::<LE>(zone_id).unwrap();
    packet
}

fn encode_mob_null_packet(index: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(3);
    packet.push(0x03u8);
    packet.write_u16::<LE>(index).unwrap();
    packet
}

fn encode_mob_packet(index: u16, pointer: u64, mut mob_data: Vec<u8>) -> Vec<u8> {

    let mut packet = Vec::with_capacity(mob_data.len() + 19);
    packet.push(0x02);
    let mut cursor = Cursor::new(&mut packet);
    cursor.set_position(1);
    cursor.write_u16::<LE>(index).unwrap();
    cursor.write_u64::<LE>(pointer).unwrap();
    cursor.write_u64::<LE>(mob_data.len() as u64).unwrap();
    packet.extend(mob_data.drain(..));
    packet

}

fn encode_target_packet(target: Target) -> Vec<u8> {
    let mut packet: Vec<u8> = Vec::with_capacity(25);
    let mut cursor = Cursor::new(&mut packet);
    cursor.write_u8(0x04).unwrap();
    cursor.set_position(1);
    cursor.write_u64::<LE>(target.target).unwrap();
    cursor.write_u64::<LE>(target.hover_target).unwrap();
    cursor.write_u64::<LE>(target.focus_target).unwrap();
    packet
}