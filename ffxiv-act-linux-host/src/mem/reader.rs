use std::fs::File;

use std::collections::HashMap;

use std::thread::{spawn, JoinHandle, sleep};
use crate::serde_json::from_reader;
use crate::mem::{Signatures, Signature};

use crate::read_process_memory::{Pid, TryIntoProcessHandle, ProcessHandle, CopyAddress};
use crate::utils;
use crate::proc_maps::{get_process_maps};
use byteorder::{LittleEndian, ReadBytesExt, ByteOrder};
use std::io::Cursor;
use std::time::Duration;
use crate::mem::packets::SyncPacket;

use std::sync::mpsc::Sender;
use crate::mem::models::{Combatant, Target};

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

pub enum MemErrorType {
    OpeningSignatureFile,
    ReadingSignatureFile,
    FindingSignature(Vec<SignatureType>),

}

fn open_sig_file() -> Result<File, MemErrorType> {
    File::open("signatures_64.json").map_err(|_| MemErrorType::OpeningSignatureFile)
}

fn read_signatures(f: File) -> Result<Signatures, MemErrorType> {
    from_reader(f).map_err(|_| MemErrorType::ReadingSignatureFile)
}

type SignatureMap = HashMap<SignatureType, usize>;

struct SignatureResult {
    pub signature_type: SignatureType,
    pub signature: Option<usize>,
}

fn scan_signatures(sigs: Signatures, ffxiv: &Pid) -> Result<SignatureMap, MemErrorType> {
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

    let (signature_map_res, signature_map_fail): (Vec<SignatureResult>, Vec<SignatureResult>) = sigs_to_scan
        .into_iter()
        .map(|a| {
            let sig = scan(*ffxiv, a.1, max_sig_len);
            SignatureResult{ signature_type: a.0, signature: sig }
        })
        .partition(|sig_res| sig_res.signature.is_some());

    if signature_map_fail.len() == 0 {
        let map = signature_map_res.into_iter().map(|sig_res| (sig_res.signature_type, sig_res.signature.unwrap()))
            .collect::<SignatureMap>();
        println!("[MEM] Found all memory signatures");
        Ok(map)
    } else {
        let failed = signature_map_fail.into_iter()
            .map(|sig_res| sig_res.signature_type)
            .collect::<Vec<_>>();
        Err(MemErrorType::FindingSignature(failed))
    }

//    Ok(signature_map)

}


pub fn run_reader(sender: Sender<SyncPacket>, ffxiv: Pid) -> Result<JoinHandle<()>, MemErrorType> {

    open_sig_file()
        .and_then(read_signatures)
        .and_then(|a| scan_signatures(a, &ffxiv))
        .and_then(|signature_map| {
            Ok(spawn(move || {
                let sigs = signature_map;
                let ffxiv = ffxiv;
                let sender = sender;
                let base_addr = sigs.get(&SignatureType::ZoneID).unwrap();
                'mem: loop {
                    // ZONE
                    let zone = read_zone_id(*base_addr, &ffxiv);
                    if let Err(_) = sender.send(SyncPacket::ZoneID(zone)) { break 'mem; }


                    if zone != 0 {
                        // MOB ARRAY

                        let mob_array_ptr = sigs.get(&SignatureType::MobArray).unwrap();
                        for i in 0..421usize {
                            let mob_opt = read_mob(*mob_array_ptr, i, &ffxiv);
                            if let Some((this_ptr, combatant)) = mob_opt {
                                let com = combatant.binary_serialize_compressed();
                                if let Err(_) = sender.send(SyncPacket::MobUpdate(i as u16, this_ptr as u64, com)) {
                                    break 'mem;
                                }
                            } else {
                                if let Err(_) = sender.send(SyncPacket::MobNull(i as u16)){
                                    break 'mem;
                                }
                            }
                        }

                        // TARGET
                        if zone != 0 {
                            let target_sig = sigs.get(&SignatureType::Target).unwrap();
                            let targets = read_target(*target_sig, &ffxiv);
                            if let Err(_) = sender.send(SyncPacket::Target(targets)) { break 'mem; }
                        }
                    }

                    sleep(Duration::from_millis(10));
                }
        }))




    })

}

fn read_signature(signature: usize, ffxiv: &Pid) -> usize {
    let copy = read_process_memory::copy_address(signature, 4, ffxiv as &Pid).unwrap();
    let mut cur = Cursor::new(copy);
    let offset = cur.read_u32::<LittleEndian>().unwrap();
    offset as usize + signature + 4
}

fn read_target(signature: usize, ffxiv: &Pid) -> Target {
    let target = read_signature(signature, ffxiv);
    let target_bin = read_process_memory::copy_address(target, 512, &ffxiv as &Pid).unwrap();
    Target::from_ffxiv_slice(target_bin)
}

fn read_mob(signature: usize, index: usize, ffxiv: &Pid) -> Option<(u64, Combatant)> {

    let mob_array = read_signature(signature, ffxiv);
    let mob_ptr_ptr = mob_array + 8 * index;
    let mob_ptr_vec = read_process_memory::copy_address(mob_ptr_ptr, 8, &ffxiv as &Pid).unwrap();
    let mob_ptr = LittleEndian::read_u64(mob_ptr_vec.as_slice()) as usize;
    if mob_ptr != 0 {
        let mob_data = read_process_memory::copy_address(mob_ptr, 11520, &ffxiv as &Pid).unwrap();
        Some((mob_ptr as u64, Combatant::from_slice(mob_data.as_slice())))
    } else {
        None
    }
}

fn read_zone_id(signature: usize, ffxiv: &Pid) -> u32 {
    let zone_id_addr= read_signature(signature, ffxiv);
    let zone_id = read_process_memory::copy_address(zone_id_addr, 4, &ffxiv as &Pid).unwrap();
    let mut cur = Cursor::new(zone_id);
    cur.read_u32::<LittleEndian>().unwrap()
}
//
//fn find_ffxiv() -> Option<Pid> {
//
//    let pid = utils::find_ffxiv()
//    pid as Pid
//}

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