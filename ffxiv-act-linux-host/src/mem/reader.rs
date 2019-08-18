use std::fs::File;
use std::io::prelude::*;

use std::collections::HashMap;

use std::thread::{spawn, JoinHandle, sleep};
use crate::serde_json::from_reader;
use crate::mem::{Signatures, Signature};

use crate::read_process_memory::{Pid, TryIntoProcessHandle, ProcessHandle, CopyAddress};
use crate::utils;
use crate::proc_maps::{get_process_maps, MapRange};
use std::process::Command;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;
use std::time::Duration;
use crate::mem::packets::SyncPacket;

use std::sync::mpsc::Sender;

const SCAN_SIZE: usize = 65536;

#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub enum SignatureType {
    Target,
    ChatLog,
    MobArray,
    PartyList,
    ServerTime,
    ZoneID,
    Player,
}

pub fn run_reader(sender: Sender<SyncPacket>) -> JoinHandle<()> {

    let mut file = File::open("signatures_64.json").expect("Couldn't open signature file");
    let sigs: Signatures = from_reader(&mut file).expect("Unable to read/parse signature file");
    let sigs_to_scan = vec![
        (SignatureType::Target, sigs.get_target()),
        (SignatureType::ChatLog, sigs.get_chat_log()),
        (SignatureType::MobArray, sigs.get_mob_array()),
        (SignatureType::PartyList, sigs.get_party_list()),
        (SignatureType::ServerTime, sigs.get_server_time()),
        (SignatureType::ZoneID, sigs.get_zone_id()),
        (SignatureType::Player, sigs.get_player())
    ];

    let max_sig_len = sigs_to_scan.iter().map(|a| a.1.signature_bytes.len()).max().unwrap();

    let ffxiv = find_ffxiv();

    println!("{:?}", sigs_to_scan[1].1);

    let signature_map = sigs_to_scan
        .into_iter()
        .map(|a| {
            let ret = (a.0, scan(ffxiv, a.1, max_sig_len).expect(format!("Unable to find signature for {:?}", a.0).as_str()));
            println!("Found signature for {:?}", ret.0);
            ret
        })
        .collect::<HashMap<_,_>>();

    println!("All signatures found.");

    spawn(move || {
        let sigs = signature_map;
        let ffxiv = ffxiv;
        let sender = sender;
        let mut cur_zone = 0u32;
        let base_addr = sigs.get(&SignatureType::ZoneID).unwrap();
        loop {

            let zone = read_zone_id(*base_addr, &ffxiv);
            if cur_zone != zone {
                println!("Zone changed: {}", zone);
                sender.send(SyncPacket::ZoneID(zone)).expect("Failed to send sync to host thread");
                cur_zone = zone;
            }
            sleep(Duration::from_millis(10));
        }
    })

}

fn read_signature(signature: usize, ffxiv: &Pid) -> usize {
    let copy = read_process_memory::copy_address(signature, 4, ffxiv as &Pid).unwrap();
    let mut cur = Cursor::new(copy);
    let offset = cur.read_u32::<LittleEndian>().unwrap();
    offset as usize + signature + 4
}

fn read_zone_id(signature: usize, ffxiv: &Pid) -> u32 {
    let zone_id_addr= read_signature(signature, ffxiv);
    let zone_id = read_process_memory::copy_address(zone_id_addr, 4, &ffxiv as &Pid).unwrap();
    let mut cur = Cursor::new(zone_id);
    cur.read_u32::<LittleEndian>().unwrap()
}

fn find_ffxiv() -> Pid {
    let output = Command::new("pgrep")
        .arg("ffxiv_dx11.exe")
        .output()
        .expect("Unable to start pgrep to find ffxiv.");
    if !output.status.success() {
        panic!("Unable to find ffxiv.");
    }
    let mut str_pid = String::from_utf8(output.stdout).expect("Unable to parse pgrep output");
    str_pid.remove(str_pid.len() - 1);
    let pid = str_pid.parse::<i32>().expect("Unable to parse pgrep output");
    pid as Pid
}

impl std::fmt::Debug for SignatureType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SignatureType::Target => write!(f, "[Target]"),
            SignatureType::ChatLog => write!(f, "[ChatLog]"),
            SignatureType::MobArray => write!(f, "[MobArray]"),
            SignatureType::PartyList => write!(f, "[PartyList]"),
            SignatureType::ServerTime => write!(f, "[ServerTime]"),
            SignatureType::ZoneID => write!(f, "[ZoneID]"),
            SignatureType::Player => write!(f, "[Player]"),
        }
    }
}

fn scan(pid: Pid, signature: Signature, max_sig_len: usize) -> Option<usize> {
    let mut scan_buffer = [0u8; SCAN_SIZE];
    let handle = pid.try_into_process_handle() as std::io::Result<ProcessHandle>;
    if handle.is_err() {
        eprintln!("Unable to open process handle.");
        return None;
    }
    let handle = handle.unwrap();

    let maps = get_process_maps(pid);
    if maps.is_err() {
        eprintln!("Unable to read process maps");
        return None;
    }
    let maps = maps.unwrap();
    let mut ret: Option<usize> = None;
    'outer: for addr_range  in maps.iter()
        .filter(|s| s.is_read() && s.size() > 0 && s.filename().is_none() && s.is_exec() && !s.is_write()) {

        let mut addr = addr_range.start();
        let end = addr + addr_range.size();
        let mut buffer_range = 0usize..SCAN_SIZE;
        loop {
            let res = (&handle).copy_address(addr, &mut scan_buffer[buffer_range.clone()]);
            if let Ok(_) = res {
                if let Some(x) = utils::find_subsequence(&scan_buffer[buffer_range.clone()], signature.signature_bytes.as_ref(), signature.wildcard_ranges.as_ref()) {
//                    println!("Found at {:x?}, offset {}, addr {}", x + addr, x, addr);
                    ret = Some(x + addr + signature.signature_bytes.len());
                    break 'outer;
                }
                addr += SCAN_SIZE - max_sig_len;
                if addr > end {
                    continue 'outer;
                }
                if addr + SCAN_SIZE > end {
                    buffer_range.end = SCAN_SIZE - (addr + SCAN_SIZE - end);
                }
            } else if let Err(x) = res {
                eprintln!("Error at addr {:x?}: {:?}", addr, x);
                ret = None;
                break 'outer;
            }
        }

    }

    ret
}