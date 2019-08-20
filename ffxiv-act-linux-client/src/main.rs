extern crate bincode;
extern crate flate2;
extern crate byteorder;

use std::sync::mpsc;
use std::net::{TcpStream, UdpSocket};
use std::fs::File;
use std::io::prelude::*;
use std::thread;

use serde_json::from_reader;
use serde::Deserialize;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};

use std::net::ToSocketAddrs;

mod models;
mod internal_models;

use models::*;
use std::collections::HashMap;
use crate::internal_models::Combatant;
use std::io::Cursor;
use std::fmt::Display;
use std::time::Duration;
//
//static mut SIGNATURE: [u8; 156] = [
//    // ZoneID
//    0xf3,0x0f,0x10,0x8d,0x08,0x04,0x00,0x00,0x4c,0x8d,0x85,0x58,0x06,0x00,0x00,0x0f,0xb7,0x05,
//    0x00, 0x00, 0x00, 0x00,
//    // Target
//    0x41,0xbc,0x00,0x00,0x00,0xe0,0x41,0xbd,0x01,0x00,0x00,0x00,0x49,0x3b,0xc4,0x75,0x55,0x48,0x8d,0x0d,
//    0x04,0x00,0x00,0x00,
//    // ChatLog
//    0xe8,0x00,0x00,0x00,0x00,0x85,0xc0,0x74,0x0e,0x48,0x8b,0x0d,0x00,0x00,0x00,0x00,0x33,0xD2,0xE8,0x00,0x00,0x00,0x00,0x48,0x8b,0x0d,
//    0x04,0x00,0x00,0x00,
//    // MobArray
//    0x48,0x8b,0x42,0x08,0x48,0xc1,0xe8,0x03,0x3d,0xa7,0x01,0x00,0x00,0x77,0x24,0x8b,0xc0,0x48,0x8d,0x0d,
//    0x04,0x00,0x00,0x00,
//    // PartyList
//    0x48,0x8D,0x7C,0x24,0x20,0x66,0x66,0x0F,0x1F,0x84,0x00,0x00,0x00,0x00,0x00,0x48,0x8B,0x17,0x48,0x8D,0x0D,
//    0x04,0x00,0x00,0x00,
//    // ServerTime
//    0x0f,0xb7,0xc0,0x89,0x47,0x10,0x48,0x8b,0x0d,
//    0x04,0x00,0x00,0x00,
//    // Player
//    0x83,0xf9,0xff,0x74,0x12,0x44,0x8b,0x04,0x8e,0x8b,0xd3,0x48,0x8d,0x0d,
//    0x04,0x00,0x00,0x00,
//];

static mut ALL_MEMORY: AllMemory = AllMemory::create();

//const ZONE_PTR_INDEX: usize = 156;
//const TARGET_PTR_INDEX: usize = 42;
//const CHATLOG_PTR_INDEX: usize = 72;
//const MOBARRAY_PTR_INDEX: usize = 96;
//const PARYLIST_PTR_INDEX: usize = 121;
//const SERVERTIME_PTR_INDEX: usize = 134;
//const PLAYER_PTR_INDEX: usize = 152;

fn handle_zone_packet<R: Read>(tcp: &mut R) {
    let mut zone_buffer = [0u8; 4];
    tcp.read_exact(&mut zone_buffer).expect("malformed zone packet");
    let zone = LittleEndian::read_u32(&zone_buffer);
    println!("new zone: {}", zone);
    unsafe { set_zone(zone); }
}

fn handle_mob_null_packet<R: ReadBytesExt>(tcp: &mut R, mob_array_heap: &mut HashMap<u16, (u64, Box<[u8; 11520]>)>) {
    let index = tcp.read_u16::<LittleEndian>().expect("malformed mob delete packet");
    if index >= 421 {
        panic!("malformed mob delete packet: oob");
    }
    unsafe {
        ALL_MEMORY.mob_array.data[index as usize] = 0u64;
    }
    mob_array_heap.remove(&index);
}

fn handle_mob_packet<R: ReadBytesExt>(tcp: &mut R, mob_array_heap: &mut HashMap<u16, (u64, Box<[u8; 11520]>)>) {
    let index = tcp.read_u16::<LittleEndian>().expect("malformed mob packet");

    let pointer = tcp.read_u64::<LittleEndian>().expect("malformed mob packet");
    let len = tcp.read_u64::<LittleEndian>().expect("malformed mob packet");
    let mut data = vec![0u8; len as usize];
    tcp.read_exact(data.as_mut_slice()).unwrap();
    let combatant = Combatant::deserialize_binary_compressed(data);
    if index == 0 {
//        println!("{:x?}", combatant.id);
    }
    if mob_array_heap.contains_key(&index) {
        let (heap_ptr, mob) = mob_array_heap.get_mut(&index).unwrap();
        if *heap_ptr != pointer {
            mob_array_heap.remove(&index);
            let new_mob = Box::new(combatant.as_ffxiv_array());
            unsafe {
                ALL_MEMORY.mob_array.data[index as usize] = (new_mob.as_ref() as *const [u8; 11520]) as u64
            }
            mob_array_heap.insert(index, (pointer, new_mob));
        } else {
            let mut cursor = Cursor::new(mob.as_mut().as_mut());
            cursor.write_all(combatant.as_ffxiv_array().as_ref()).unwrap();
        }
    } else {
        let new_mob = Box::new(combatant.as_ffxiv_array());
        unsafe {
            ALL_MEMORY.mob_array.data[index as usize] = (new_mob.as_ref() as *const [u8; 11520]) as u64;
        }
        mob_array_heap.insert(index, (pointer, new_mob));
    }
}

enum ThreadControlMsg {
    Ending(ThreadType),
    UnableToConnect(ThreadType),
    ReadTimeOut(ThreadType),
}

enum ThreadType {
    FFXIV,
    Mem,
}

impl Display for ThreadType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ThreadType::Mem => write!(f, "[Memory]"),
            ThreadType::FFXIV => write!(f, "[FFXIV]"),
        }
    }
}

fn main() {
    'outer: loop {
        unsafe { setup_memory(); }

        let config: Config = {
            let mut file = File::open("config.json").expect("Unable to open config file");
            from_reader(&mut file).expect("Unable to read / parse config file")
        };


        let (thread_ctl_tx, thread_ctl_rx) = mpsc::channel();

        start_ffxiv_client(config.address.clone(), thread_ctl_tx.clone());
        start_mem_sync_client(config.address, thread_ctl_tx.clone());

        for msg in thread_ctl_rx {
            match msg {
                ThreadControlMsg::Ending(t) => {
                    println!("{} channel disconnected.", t);
                    break 'outer;
                },
                ThreadControlMsg::UnableToConnect(t) => {
                    println!("{} -- unable to connect to host.", t);
                    break 'outer;
                },
                ThreadControlMsg::ReadTimeOut(t) => {
                    println!("{} -- read timed out.", t);
                    break 'outer;
                }
            }
        }
        unsafe {
            println!("Memory sync bank ptr: {:p}", &ALL_MEMORY as *const AllMemory)
        }
    }



}

fn start_ffxiv_client(addr: String, thread_ctl: mpsc::Sender<ThreadControlMsg>) {
    thread::spawn(move || {
        let mut addr = addr.to_socket_addrs().unwrap().next().unwrap();
        addr.set_port(54992);
        if let Ok(mut tcp_ffxiv) = TcpStream::connect(addr) {
            let mut byte_buffer_ffxiv = [0u8; 32768];
            loop {
                let read = tcp_ffxiv.read(&mut byte_buffer_ffxiv).unwrap();
                if read == 0 {
                    break;
                }
                println!("[FFXIV][DEBUG] Read {} bytes.", read);
            }
            thread_ctl.send(ThreadControlMsg::Ending(ThreadType::FFXIV)).unwrap();
        } else {
            thread_ctl.send(ThreadControlMsg::UnableToConnect(ThreadType::FFXIV)).unwrap();
        }
    });
}

fn start_mem_sync_client(addr: String, thread_ctl: mpsc::Sender<ThreadControlMsg>) {
    thread::spawn(move || {
        let mut mob_array_heap: HashMap<u16, (u64, Box<[u8; 11520]>)> = HashMap::new();
        let mut buffer = [0u8; 12000];
        let addr = {
            if let Ok(mut addrs) = addr.to_socket_addrs() {
                if let Some(a) = addrs.next() {
                    a
                } else {
                    thread_ctl.send(ThreadControlMsg::UnableToConnect(ThreadType::Mem)).unwrap();
                    return;
                }
            } else {
                thread_ctl.send(ThreadControlMsg::UnableToConnect(ThreadType::Mem)).unwrap();
                return;
            }
        };

        let udp_client = UdpSocket::bind("192.168.122.149:30005").unwrap();
        udp_client.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        udp_client.connect(addr).unwrap();
        udp_client.send(&[0u8,0,0,0,0,0,0,0]).unwrap();
        'mem: loop {
            match udp_client.recv(&mut buffer) {
                Ok(num) => {
                    if num > 0 {
                        let mut cursor = Cursor::new(&buffer[1..]);
                        match buffer[0] {
                            0x01 => handle_zone_packet(&mut cursor),
                            0x02 => handle_mob_packet(&mut cursor, &mut mob_array_heap),
                            0x03 => handle_mob_null_packet(&mut cursor, &mut mob_array_heap),
                            _ => panic!("Unknown packet type"),
                        }
                    } else {
                        break 'mem;
                    }
                },
                Err(ref err) => {
                    let k = err.kind();
                    use std::io::ErrorKind as EK;
                    if k == EK::WouldBlock || k == EK::TimedOut {
                        thread_ctl.send(ThreadControlMsg::ReadTimeOut(ThreadType::Mem)).unwrap();
                        break 'mem;
                    }
                }
            }
        }
        thread_ctl.send(ThreadControlMsg::Ending(ThreadType::Mem)).unwrap();
    });
}

#[derive(Deserialize)]
struct Config {
    pub address: String,
}

unsafe fn setup_memory() {
    SERVER_2.ptr3 = (&SERVER_3) as *const ServerTimePart3 as u64;
    SERVER_1.ptr2 = (&SERVER_2) as *const ServerTimePart2 as u64;
    ALL_MEMORY.server_time.ptr = (&SERVER_1) as *const ServerTimePart1 as u64;
}

unsafe fn set_zone(zone: u32) {
    ALL_MEMORY.zone_id.data = zone;
}