
use std::io::prelude::*;
use std::net::{TcpListener, UdpSocket};

use std::thread;

use std::error;
use std::fmt;
use std::thread::JoinHandle;

use std::sync::mpsc::{Sender, Receiver, channel};
use crate::mem::packets::{SyncPacket, EncodePacket};
use std::time::Duration;


pub fn run_server() -> (Sender<SyncPacket>, JoinHandle<Result<(), ServerError>>) {
    let (tx, rx) = channel();
    (tx, thread::spawn(move || {
        let rx = rx;
        let udp = UdpSocket::bind("0.0.0.0:7262").map_err(|_| ServerError::Binding)?;
        udp.set_write_timeout(Some(Duration::from_secs(10))).unwrap();
        let (_, client) = udp.recv_from(&mut [0u8;5]).unwrap();
        println!("[MEM] Client connected from {}", client);
        udp.connect(client).unwrap();
        for sync in rx {
            let buf = sync.encode_packet();
            match udp.send(buf.as_slice()) {
                Ok(_) => {
                },
                Err(_) => break,
            }
        }
        Ok(())

//        let listener = TcpListener::bind("0.0.0.0:7262").map_err(|_| ServerError::Binding)?;
//        Ok::<(), ServerError>(()).and_then(|_| {
//            let (mut inc, _) = listener.accept().map_err(|a| ServerError::Connecting(a))?;
//            for sync in rx {
//                println!("Sending sync packet");
//                let buf = sync.encode_packet();
//                inc.write(&buf[..]);
//            }
//            println!("ending");
//            Ok(())
//        })
    }))
}

pub enum ServerError {
    Binding,
    Connecting(std::io::Error)
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServerError::Binding => write!(f, "unable to bind to server"),
            ServerError::Connecting(e) => write!(f, "unable to connect: {:?}", e),
        }
    }
}

impl error::Error for ServerError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ServerError::Binding => None,
            ServerError::Connecting(e) => Some(e),
        }
    }
}

impl fmt::Debug for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServerError::Binding => write!(f, "unable to bind to server"),
            ServerError::Connecting(e) => write!(f, "unable to connect: {:?}", e),
        }
    }
}