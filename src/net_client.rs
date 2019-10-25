use crate::frame::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

use serde::{Deserialize, Serialize};
pub enum ToNetClientInner {
    NewFrame(Frame),
}

pub enum FromNetClientInner {
    PlayerInput(FrameEvent),
}

pub struct NetClient {
    s: Sender<ToNetClientInner>,
    r: Receiver<FromNetClientInner>,
}

impl NetClient {
    pub fn new(bind: &str) -> Self {
        let (s_to, r_to) = unbounded::<ToNetClientInner>();
        let (s_from, r_from) = unbounded::<FromNetClientInner>();
        let bind_addr = bind.to_owned();
        std::thread::spawn(move || {
            let mut stream = TcpStream::connect(bind_addr).unwrap();

            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                log::info!("Connection established!");

                //Server frame -> s_from

                // r_to -> player input to send
            }
        });
        NetClient { s: s_to, r: r_from }
    }

    pub fn collect_data_to_compute_next_frame(&mut self) {}

    pub fn send_player_inputs(&mut self, data: DataToComputeNextFrame) {}
}
