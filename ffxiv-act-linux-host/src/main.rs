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

use serde::{Deserialize};
use std::io;
use std::io::prelude::*;
use byteorder::{LittleEndian, ReadBytesExt};
use read_process_memory::{Pid, TryIntoProcessHandle, copy_address};

use std::net::TcpListener;

fn main() {

    mem::begin();


}

