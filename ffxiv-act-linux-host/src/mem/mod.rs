mod host_server;
mod reader;
mod packets;
mod models;

use crate::Deserialize;
use crate::hex;
use std::ops::Range;

/// Begins the memory portion of the interface. Starts a thread for memory reading and a thread for
/// memory synchronization to the client.
pub fn begin() {

    let (sender, host_handle) = host_server::run_server();
    let reader_handle = reader::run_reader(sender);

    match host_handle.join() {
        Ok(e) => e.unwrap(),
        Err(e) => panic!(e),
    };
    match reader_handle.join() {
        Ok(_) => {},
        Err(e) => panic!(e),
    };
}

#[derive(Deserialize)]
struct Signatures {
    target: String,
    chat_log: String,
    mob_array: String,
    party_list: String,
    server_time: String,
    zone_id: String,
    player: String,
}

#[derive(Debug)]
struct Signature {
    pub signature_bytes: Vec<u8>,
    pub wildcard_ranges: Option<Vec<Range<usize>>>,
}

impl Signatures {
    pub fn get_target(&self) -> Signature {
        self.target.parse_signature()
    }
    pub fn get_chat_log(&self) -> Signature {
        self.chat_log.parse_signature()
    }
    pub fn get_mob_array(&self) -> Signature {
        self.mob_array.parse_signature()
    }
    pub fn get_party_list(&self) -> Signature {
        self.party_list.parse_signature()
    }
    pub fn get_server_time(&self) -> Signature {
        self.server_time.parse_signature()
    }
    pub fn get_zone_id(&self) -> Signature {
        self.zone_id.parse_signature()
    }
    pub fn get_player(&self) -> Signature {
        self.player.parse_signature()
    }
}

trait ParseSignature {
    fn parse_signature(&self) -> Signature;
}

impl<S: AsRef<str>> ParseSignature for S {
    fn parse_signature(&self) -> Signature {
        let unwild = self.as_ref().replace("?", "0");
        let wildcards = self.as_ref().parse_wildcards();
        let bytes = hex::decode(unwild).expect("unable to parse signature from hex");
        Signature { signature_bytes: bytes, wildcard_ranges: wildcards }
    }
}

trait ParseWildcards {
    fn parse_wildcards(&self) -> Option<Vec<Range<usize>>>;
}

impl<S: AsRef<str>> ParseWildcards for S {
    fn parse_wildcards(&self) -> Option<Vec<Range<usize>>> {
        let mut ranges = Vec::new();
        let s = self.as_ref();
        let mut is_on_wild_range = false;
        let mut cur_range_len = 0usize;
        let mut last_question = false;
        for (i, c) in s.chars().enumerate() {
            if last_question {
                if c != '?' {
                    panic!("wildcard byte is malformed")
                }
                last_question = false;
                continue;
            }
            if c == '?' {
                if is_on_wild_range {
                    cur_range_len += 1;
                } else {
                    is_on_wild_range = true;
                    cur_range_len = 0;
                }
                last_question = true;
            } else {
                if is_on_wild_range {
                    is_on_wild_range = false;
                    let range_end = i / 2 - 1;
                    let range_start = range_end - cur_range_len;
                    let range = range_start..(range_end+1);
                    ranges.push(range);
                }
            }
        }
        if ranges.len() > 0 {
            Some(ranges)
        } else {
            None
        }
    }
}