use crate::frame::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

use serde::{Deserialize, Serialize};
pub enum ToNetServerInner {
    NewFrame(Frame),
}

pub enum FromNetServerInner {
    PlayerInput(FrameEvent),
}

pub struct NetServer {
    s: Sender<ToNetServerInner>,
    r: Receiver<FromNetServerInner>,
}

impl NetServer {
    pub fn new(bind: &str) -> Self {
        let (s_to, r_to) = unbounded::<ToNetServerInner>();
        let (s_from, r_from) = unbounded::<FromNetServerInner>();
        let bind_addr = bind.to_owned();
        std::thread::spawn(move || {
            let listener = TcpListener::bind(bind_addr).unwrap();

            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                println!("Connection established!");

                //Client msg -> s_from

                // r_to -> New frame to broadcast
            }
        });
        NetServer { s: s_to, r: r_from }
    }

    pub fn collect_remote_players_inputs(&mut self) -> Vec<FrameEvent> {
        Vec::new()
    }

    pub fn broadcast(&mut self, data: DataToComputeNextFrame) {}
}
