use std::io::prelude::*;
use std::net::{TcpListener};

use std::sync::mpsc;

use std::thread;

use crate::pcap;
use pcap::Device;

use etherparse::SlicedPacket;

use crate::NetConfig;
use std::process::Command;

pub fn start_packet_redirection(net_config: NetConfig, ffxiv: i32) -> bool {
    let interface = net_config.interface;
    let host_exclude = net_config.hostname_exclude;
    let sender_opt = start_incoming_sync_host(net_config.bind_address);
    if let Some(sender) = sender_opt {
        if let Ok(device_list) = Device::list() {
            let device_opt = device_list.into_iter().filter(|d| d.name == interface).next();
            if device_opt.is_none() {
                eprintln!("[NET] Unable to find device with name \"{}\"", interface);
                return false;
            }
            let device = device_opt.unwrap();
            println!("[NET] Attempting to capture on {}", device.name);

            let device_open_res = device.open();
            if device_open_res.is_err() {
                eprintln!("[NET] Unable to open device for network capture. Are you root?");
                return false;
            }
            let mut cap = device_open_res.unwrap();
            let src_port = get_src_port(ffxiv);
            println!("[NET] Identified FFXIV Server port as {}, capturing traffic from that port.", src_port);
            cap.filter(format!("(src port {}) && (src host not {})", src_port, host_exclude).as_str()).expect("[NET] Unable to apply filters");
            println!("[NET] Setup pcap for network redirection");
            'capture: loop {

                if let Ok(p) = cap.next() {
                    let data = p.data.to_vec();
                    let pa = SlicedPacket::from_ethernet(data.as_slice()).unwrap();

                    let pref = pa.payload;

                    if pref.len() == 0 {
                        continue;
                    } else {
                        if let Err(_) = sender.send(pref.to_vec()) {
                            break 'capture;
                        }
                    }
                } else {
                    eprintln!("[NET] Unable to get next packet! Something may have gone wrong earlier.");
                    return false;
                }
            }
            true
        } else {
            eprintln!("[NET] Unable to lookup devices. Are you root?");
            false
        }
    } else {
        eprintln!("[NET] Unable to start network sync host.");
        false
    }
}

fn start_incoming_sync_host(bind_address: String) -> Option<mpsc::Sender<Vec<u8>>> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    if let Ok(tcp) = TcpListener::bind(&bind_address) {
        println!("[NET] Opened fake ffxiv server on {}.", bind_address);
        thread::spawn(move || {
            loop {
                println!("[NET] Waiting for TCP client");
                let (mut inc, from) = tcp.accept().expect("[NET] Unable to accept connection");
                println!("[NET] TCP connection from {}", from);
                // Clear prior packets
                let mut iter = rx.try_iter();
                while let Some(_) = iter.next() {}

                // Send packets as received
                'sync: for data in &rx {
                    if let Err(_) = inc.write(&data[..]) {
                        println!("[NET] Client connection ending.");
                        break 'sync;
                    }
                }
            }
        });
        Some(tx)
    } else {
        eprintln!("[NET] Unable to bind socket on {}. Is another process using it?", bind_address);
        None
    }

}

fn get_src_port(pid: i32) -> u16 {
    use regex::Regex;
    let output = Command::new("lsof")
        .arg("-i")
        .arg("-a")
        .arg("-p")
        .arg(format!("{}", pid))
        .output().expect("Unable to get lsof");

    let lsof = String::from_utf8(output.stdout).expect("Couldn't read lsof output");
    let re = Regex::new(r":(\d+) \(ESTABLISHED\)").unwrap();
    let port_s = &re.captures_iter(lsof.as_str()).next().unwrap()[1];

    let port = port_s.parse::<u16>().expect("Couldn't parse port");

    port
}
