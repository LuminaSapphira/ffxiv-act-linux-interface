use crate::byteorder::{LittleEndian as LE, WriteBytesExt};
use crate::mem::models::Target;

#[derive(Clone)]
pub enum SyncPacket {
    ZoneID(u32),
    MobUpdate(u16, u64, Vec<u8>),
    MobNull(u16),
    Target(Target),
    ServerTime(u64),
}

pub trait EncodePacket {
    fn encode_packet(self, seq: u64) -> Vec<u8>;
}

impl EncodePacket for SyncPacket {
    fn encode_packet(self, seq: u64) -> Vec<u8> {
        let mut header = Vec::new();

        header.write_u8(get_packet_id(&self)).unwrap();
        header.write_u64::<LE>(seq).unwrap();

        let mut encoded =
        match self {
            SyncPacket::ZoneID(zone) => write_zone_packet(header, zone),
            SyncPacket::MobUpdate(index, pointer, mob_data) => write_mob_packet(header, index, pointer, mob_data),
            SyncPacket::MobNull(index) => write_mob_null_packet(header, index),
            SyncPacket::Target(target) => write_target_packet(header, target),
            SyncPacket::ServerTime(server_time) => write_server_time_packet(header, server_time),
        };
        encoded.shrink_to_fit();
        encoded
    }
}

#[inline]
fn get_packet_id(packet_data: &SyncPacket) -> u8 {
    match packet_data {
        SyncPacket::ZoneID(_) => 1,
        SyncPacket::MobUpdate(_, _, _) => 2,
        SyncPacket::MobNull(_) => 3,
        SyncPacket::Target(_) => 4,
        SyncPacket::ServerTime(_) => 5,
    }
}

fn write_zone_packet(header: Vec<u8>, zone_id: u32) -> Vec<u8> {
    let mut packet = header;
    packet.write_u32::<LE>(zone_id).unwrap();
    packet
}

fn write_server_time_packet(header: Vec<u8>, server_time: u64) -> Vec<u8> {
    let mut packet = header;
    packet.write_u64::<LE>(server_time).unwrap();
    packet
}

fn write_mob_packet(header: Vec<u8>, index: u16, pointer: u64, mob_data: Vec<u8>) -> Vec<u8> {
    let mut mob_data = mob_data;
    let mut packet = header;
    packet.write_u16::<LE>(index).unwrap();
    packet.write_u64::<LE>(pointer).unwrap();
    packet.write_u64::<LE>(mob_data.len() as u64).unwrap();
    packet.extend(mob_data.drain(..));
    packet
}

fn write_mob_null_packet(header: Vec<u8>, index: u16) -> Vec<u8> {
    let mut packet = header;
    packet.write_u16::<LE>(index).unwrap();
    packet
}

fn write_target_packet(header: Vec<u8>, target: Target) -> Vec<u8>  {
    let mut packet = header;
    packet.write_u64::<LE>(target.target).unwrap();
    packet.write_u64::<LE>(target.hover_target).unwrap();
    packet.write_u64::<LE>(target.focus_target).unwrap();
    packet
}

#[cfg(test)]
mod sync_packet_tests {

    use crate::mem::packets::*;
    use crate::mem::models::Target;

    #[test]
    fn encode_zone() {
        let zone: u32 = 641;
        let packet = SyncPacket::ZoneID(zone).encode_packet(8);
        let expected = vec![1u8,8,0,0,0,0,0,0,0,0x81,0x02,0,0];
        assert_eq!(packet.len(), expected.len());
        assert_eq!(packet, expected);
    }

    #[test]
    fn encode_mob_packet() {
        let index: u16 = 30000;
        let pointer: u64 = 5_000_000_000;
        let mob_data = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
        let packet = SyncPacket::MobUpdate(index, pointer, mob_data);
        let packet = packet.encode_packet(400);
        let expected = vec![2u8, 0x90, 0x01, 0, 0, 0, 0, 0, 0, 0x30, 0x75, 0x00, 0xf2, 0x05, 0x2a, 0x01, 0,0,0, 8, 0,0,0,0,0,0,0,0,1,2,3,4,5,6,7];
        assert_eq!(packet.len(), expected.len());
        assert_eq!(packet, expected);
    }

    #[test]
    fn encode_mob_null_packet() {
        let index: u16 = 65000;
        let packet = SyncPacket::MobNull(index);
        let packet = packet.encode_packet(12345678987654321);
        let expected = vec![3u8, 0xb1, 0xf4, 0x91, 0x62, 0x54, 0xdc, 0x2b, 0x00, 0xe8, 0xfd];
        assert_eq!(packet.len(), expected.len());
        assert_eq!(packet, expected);
    }

    #[test]
    fn encode_target_packet() {
        let target = Target {
            target: 9223372036854775807,
            hover_target: 98765432123456789,
            focus_target: 1
        };

        let packet = SyncPacket::Target(target).encode_packet(4444);
        let expected = vec![4u8, 0x5c, 0x11, 0,0,0,0,0,0, 0xff, 0xff,0xff,0xff,0xff,0xff,0xff,0x7f,0x15,0x7d,0xce,0x21,0xa3,0xe2,0x5e,0x01,0x01,0,0,0,0,0,0,0];
        assert_eq!(packet.len(), expected.len());
        assert_eq!(packet, expected);
    }

    #[test]
    fn encode_server_packet() {
        let packet = SyncPacket::ServerTime(1234);
        let packet = packet.encode_packet(987);
        let expected = vec![5u8, 0xdb, 0x03, 0,0,0,0,0,0, 0xd2, 0x04, 0,0,0,0,0,0 ];
        assert_eq!(packet.len(), expected.len());
        assert_eq!(packet, expected);
    }
}