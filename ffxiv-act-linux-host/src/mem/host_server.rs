
use std::net::{UdpSocket};

use std::thread;

use std::error;
use std::fmt;
use std::thread::JoinHandle;

use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use crate::mem::packets::{SyncPacket, EncodePacket};
use std::collections::HashMap;
use std::time::Duration;

const MEMORY_MAGIC:[u8; 8] = [7,2,6,2,2,5,4,4];
const KEEP_ALIVE_MAGIC:[u8; 8] = [123,157,225,223,116,254,178,126];

struct MemoryClient {
    pub keep_alive_sender: Sender<()>,
    pub memory_sender: Sender<SyncPacket>,
}

pub fn run_server(rx: Receiver<SyncPacket>, addr: String) -> JoinHandle<Result<(), ServerError>> {
    thread::spawn(move || {
        let rx = rx;
        let udp = UdpSocket::bind(&addr).map_err(|_| ServerError::Binding(addr.clone()))?;
        println!("[MEM] UDP memory-sync socket bound to {}", addr);
        udp.set_broadcast(true).expect("Unable to set broadcast");

        let udp_ref = Arc::new(udp);

        let client_channels = Arc::new(Mutex::new(HashMap::new()));

        let cc1 = client_channels.clone();
        let cc2 = client_channels.clone();

        let mut recv_buffer = [0u8; 8];
        thread::spawn(move || {
            for sync in rx {
                let channel_lock = cc1.lock().unwrap();
                channel_lock.values().map(|a: &MemoryClient| &a.memory_sender).for_each(|sender| {
                    sender.send(sync.clone()).expect("Memory syncpacket failed to send to a memory client");
                });
            }
        });
        loop {
            let (_, client) = udp_ref.recv_from(&mut recv_buffer).unwrap();
            if recv_buffer == MEMORY_MAGIC {
                let (udp_tx, udp_rx) = mpsc::channel();
                let (ka_tx, ka_rx) = mpsc::channel();
                cc2.lock().unwrap().insert(client.clone(), MemoryClient{ keep_alive_sender: ka_tx, memory_sender: udp_tx });
                let cc3 = cc2.clone();
                println!("[MEM] UDP memory-sync client connected from {}", client);
                let udp_ref2 = udp_ref.clone();
                thread::spawn(move || {
                    let udp_rx = udp_rx;
                    let client = client;

                    let (stop_channel_tx, stop_channnel_rx) = mpsc::channel();

                    thread::spawn(move || {
                        let ka_rx = ka_rx;
                        let mut heartbeats_missed = 0;
                        'keep_alive_chk: loop {
                            if let Some(_) = ka_rx.try_iter().next() {
                                heartbeats_missed = 0;
                            } else {
                                thread::sleep(Duration::from_secs(1));
                                heartbeats_missed += 1;
                            }

                            if heartbeats_missed == 3 {
                                stop_channel_tx.send(()).unwrap();
                                break 'keep_alive_chk;
                            }
                        }
                    });
                    let mut sync_sequence = 0u64;
                    'mem_sync: for sync in udp_rx {
                        let buf = sync.encode_packet(sync_sequence);
                        udp_ref2.send_to(buf.as_slice(), &client).unwrap();
                        if let Some(_) = stop_channnel_rx.try_iter().next() {
                            println!("[MEM] {} missed too many heartbeats, disconnecting.", client);
                            cc3.lock().unwrap().remove(&client);
                            break 'mem_sync;
                        }
                        sync_sequence += 1;
                    }
                });
            } else if recv_buffer == KEEP_ALIVE_MAGIC {
                if let Some(mem_client) = cc2.lock().unwrap().get(&client) {
                    mem_client.keep_alive_sender.send(()).expect("Keep alive signal failed to send");
                }
            }
        }

    })
}

pub enum ServerError {
    Binding(String),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServerError::Binding(addr) => write!(f, "Unable to bind socket to {}.", addr),
        }
    }
}

impl error::Error for ServerError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ServerError::Binding(_) => None,
        }
    }
}

impl fmt::Debug for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServerError::Binding(addr) => write!(f, "Unable to bind socket to {}.", addr),
        }
    }
}