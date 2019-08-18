use crate::read_process_memory::{Pid, TryIntoProcessHandle, ProcessHandle, CopyAddress};
use crate::utils;
use crate::proc_maps::{get_process_maps, MapRange};

//pub struct Signatures {
//
//}
//
pub enum SignatureType {
//    Target,
//    ChatLog,
//    MobArray,
//    PartyList,
//    ServerTime,
    ZoneID,
//    Player,
}

const SCAN_SIZE: usize = 65536;

// TODO make dynamic
const MAX_SIG_LEN: usize = 18;
const SIG: [u8; MAX_SIG_LEN] = [0xf3,0x0f,0x10,0x8d,0x08,0x04,0x00,0x00,0x4c,0x8d,0x85,0x58,0x06,0x00,0x00,0x0f,0x00,0x05];

pub fn scan(pid: Pid, sig_type: SignatureType) -> std::io::Result<usize> {
    let mut scan_buffer = [0u8; SCAN_SIZE];
    let handle = pid.try_into_process_handle()? as ProcessHandle;

    let maps = get_process_maps(pid)?;
    'outer: for addr_range  in maps.iter()
        .filter(|s| s.is_read() && s.size() > 0 && s.filename().is_none()) {

        let mut addr = addr_range.start();
        let end = addr + addr_range.size();
        let mut buffer_range = 0usize..SCAN_SIZE;
        loop {
            let res = (&handle).copy_address(addr, &mut scan_buffer[buffer_range.clone()]);
            if let Ok(_) = res {
                if let Some(x) = utils::find_subsequence(&scan_buffer[buffer_range.clone()], &SIG, Some(vec![16usize..17usize])) {
                    println!("Found at {:x?}, offset {}, addr {}", x + addr, x, addr);
                    break 'outer;
                }
                addr += SCAN_SIZE - MAX_SIG_LEN;
                if addr > end {
                    continue 'outer;
                }
                if addr + SCAN_SIZE > end {
                    buffer_range.end = SCAN_SIZE - (addr + SCAN_SIZE - end);
                }
            } else if let Err(x) = res {
                eprintln!("Error at addr {:x?}: {:?}", addr, x);
                break 'outer;
            }
        }

    }

    Ok(0usize)
}