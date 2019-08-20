//mod signatures;
mod utils;
mod mem;
mod net;

extern crate byteorder;
extern crate read_process_memory;
extern crate proc_maps;
extern crate serde;
extern crate serde_json;
extern crate hex;
extern crate pcap;
extern crate etherparse;
extern crate bincode;
extern crate flate2;


use serde::{Deserialize};
use serde_json::from_reader;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use byteorder::{LittleEndian, ReadBytesExt};
use read_process_memory::{Pid, TryIntoProcessHandle, copy_address};

use std::net::TcpListener;

use std::thread;

fn main() {
    let config: Config = {
        let mut file = File::open("config.json").expect("Couldn't open config file");
        from_reader(&mut file).expect("Couldn't read / parse config file.")
    };

    let mem = thread::spawn(move || {
        mem::begin()
    });

    let net = thread::spawn(move || {
        net::start_packet_redirection(config.net_config)
    });

    mem.join().expect("Error in mem thread");
    net.join().expect("Error in network thread");


}

#[derive(Deserialize)]
pub struct Config {
    pub net_config: NetConfig
}

#[derive(Deserialize)]
pub struct NetConfig {
    pub interface: String,
    pub hostname_exclude: String,
}
