extern crate bincode;
extern crate flate2;
extern crate byteorder;

use std::sync::{mpsc, Arc};
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
use std::time::{Duration, Instant};
use std::error::Error;


static mut ALL_MEMORY: AllMemory = AllMemory::create();

fn handle_zone_packet<R: Read>(tcp: &mut R) {
    let mut zone_buffer = [0u8; 4];
    tcp.read_exact(&mut zone_buffer).expect("malformed zone packet");
    let zone = LittleEndian::read_u32(&zone_buffer);
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

fn handle_target_packet<R: ReadBytesExt>(data: &mut R, mob_array_heap: &mut HashMap<u16, (u64, Box<[u8; 11520]>)>) {

    let host_target = data.read_u64::<LittleEndian>().unwrap();
    let host_hover_target = data.read_u64::<LittleEndian>().unwrap();
    let host_focus_target = data.read_u64::<LittleEndian>().unwrap();
    let target = get_client_mob_pointer_from_host(host_target, mob_array_heap);
    let hover_target = get_client_mob_pointer_from_host(host_hover_target, mob_array_heap);
    let focus_target = get_client_mob_pointer_from_host(host_focus_target, mob_array_heap);
    unsafe {
        ALL_MEMORY.target.target_data.target = target;
        ALL_MEMORY.target.target_data.hovertarget = hover_target;
        ALL_MEMORY.target.target_data.focustarget = focus_target;
    }

}

fn get_client_mob_pointer_from_host(host_pointer: u64, mob_array_heap: &mut HashMap<u16, (u64, Box<[u8; 11520]>)>) -> u64 {
    if host_pointer != 0 {
        let (_, mob) = mob_array_heap.values().find(|(ptr, _)| *ptr == host_pointer).unwrap();
        let mob_ref = mob.as_ref();
        (mob_ref as *const [u8; 11520]) as u64
    } else { 0 }
}

enum ThreadControlMsg {
    Ending(ThreadType),
    Error(ThreadType, Box<dyn Error + Send>),
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
            ThreadType::Mem => write!(f, "[MEM]"),
            ThreadType::FFXIV => write!(f, "[NET]"),
        }
    }
}

fn main() {
    'outer: loop {
        unsafe { setup_memory(); }

        let config: Config = {
            let file_res = File::open("config.json");
            if let Ok(file) = file_res {
                match from_reader(file) {
                    Ok(conf) => conf,
                    Err(e) => {
                        eprintln!("Unable to read / parse config file: {}", e.description());
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Unable to open config file.");
                std::process::exit(1);
            }
        };


        let (thread_ctl_tx, thread_ctl_rx) = mpsc::channel();

        start_ffxiv_client(config.net_address, thread_ctl_tx.clone());
        start_mem_sync_client(config.mem_address, thread_ctl_tx.clone());

        for msg in thread_ctl_rx {
            match msg {
                ThreadControlMsg::Ending(t) => {
                    println!("{} Channel disconnected.", t);
                    break 'outer;
                },
                ThreadControlMsg::UnableToConnect(t) => {
                    println!("{} Unable to connect to host.", t);
                    break 'outer;
                },
                ThreadControlMsg::ReadTimeOut(t) => {
                    println!("{} Read timed out.", t);
                    break 'outer;
                },
                ThreadControlMsg::Error(t, err) => {
                    println!("{} Errored!", t);
                    eprintln!("{:?}", err);
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
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        if let Ok(mut tcp_ffxiv) = TcpStream::connect(addr) {
            println!("[NET] Connected FFXIV-passthrough client.");
            let mut byte_buffer_ffxiv = [0u8; 32768];
            loop {
                let read = tcp_ffxiv.read(&mut byte_buffer_ffxiv).unwrap();
                if read == 0 {
                    break;
                }
            }
            thread_ctl.send(ThreadControlMsg::Ending(ThreadType::FFXIV)).unwrap();
        } else {
            thread_ctl.send(ThreadControlMsg::UnableToConnect(ThreadType::FFXIV)).unwrap();
        }
    });
}

const MEMORY_MAGIC:[u8; 8] = [7,2,6,2,2,5,4,4];
const KEEP_ALIVE_MAGIC:[u8; 8] = [123,157,225,223,116,254,178,126];

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

        let udp_client = Arc::new(UdpSocket::bind("0.0.0.0:0").unwrap());
        udp_client.set_nonblocking(true).unwrap();
        udp_client.connect(addr).unwrap();
        udp_client.send(&MEMORY_MAGIC).unwrap();
        println!("[MEM] Opened UDP memory-sync socket and attempting to connect to host...");
        let mut has_recv = false;
        let udp2 = udp_client.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(500));
                udp2.send(&KEEP_ALIVE_MAGIC).unwrap();
            }
        });

        let start_instant = Instant::now();
        let mut last_recv: Option<Instant> = None;
        let mut last_seq: u64 = 0;
        'mem: loop {
            match udp_client.recv(&mut buffer) {
                Ok(num) => {
                    if num > 0 {
                        if !has_recv {
                            println!("[MEM] UDP memory-sync connection validated.");
                            has_recv = true;
                        }
                        last_recv = Some(Instant::now());
                        let mut cursor = Cursor::new(&buffer[1..]);
                        let seq = cursor.read_u64::<LittleEndian>();
                        if seq.is_err() {
                            thread_ctl.send(ThreadControlMsg::Error(ThreadType::Mem, Box::new(seq.unwrap_err()))).unwrap();
                            break 'mem;
                        }
                        let seq = seq.unwrap();
                        if last_seq > seq {
                            last_seq = seq;
                            match buffer[0] {
                                0x01 => handle_zone_packet(&mut cursor),
                                0x02 => handle_mob_packet(&mut cursor, &mut mob_array_heap),
                                0x03 => handle_mob_null_packet(&mut cursor, &mut mob_array_heap),
                                0x04 => handle_target_packet(&mut cursor, &mut mob_array_heap),
                                _ => panic!("Unknown packet type"),
                            }
                        }
                    } else {
                        break 'mem;
                    }
                },
                Err(ref err) => {
                    let k = err.kind();
                    use std::io::ErrorKind as EK;
                    if k == EK::WouldBlock {
                        thread::sleep(Duration::from_millis(5));
                        if !has_recv && start_instant.elapsed().as_secs() >= 5 {
                            thread_ctl.send(ThreadControlMsg::UnableToConnect(ThreadType::Mem)).unwrap();
                            break 'mem;
                        } else if has_recv && last_recv.unwrap().elapsed().as_secs() >= 1 {
                            thread_ctl.send(ThreadControlMsg::ReadTimeOut(ThreadType::Mem)).unwrap();
                            break 'mem;
                        }
                    }
                }
            }
        }
        thread_ctl.send(ThreadControlMsg::Ending(ThreadType::Mem)).unwrap();
    });
}

#[derive(Deserialize)]
struct Config {
    pub mem_address: String,
    pub net_address: String,
}

unsafe fn setup_memory() {
    SERVER_2.ptr3 = (&SERVER_3) as *const ServerTimePart3 as u64;
    SERVER_1.ptr2 = (&SERVER_2) as *const ServerTimePart2 as u64;
    ALL_MEMORY.server_time.ptr = (&SERVER_1) as *const ServerTimePart1 as u64;
}

unsafe fn set_zone(zone: u32) {
    ALL_MEMORY.zone_id.data = zone;
}