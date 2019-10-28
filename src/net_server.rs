use crate::frame::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use spin_sleep::LoopHelper;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BindState {
    Unknown,
    Success,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct NetServerInfo {
    bind_state: BindState,
    number_of_client_connected: usize,
}

pub enum ToNetServerInner {
    DataToComputeNextFrame(DataToComputeNextFrame),
}

pub enum FromNetServerInner {
    PlayerInputs(Vec<FrameEventFromPlayer>),
}

pub struct NetServer {
    s_inner: Sender<ToNetServerInner>,
    r_inner: Receiver<FromNetServerInner>,
    info: NetServerInfo,
    r_info: Receiver<NetServerInfo>,
}

impl NetServer {
    pub fn new(bind: &str) -> Self {
        let (s_to, r_to) = unbounded::<ToNetServerInner>();
        let (s_from, r_from) = unbounded::<FromNetServerInner>();

        let (s_info, r_info) = unbounded::<NetServerInfo>();
        let bind_addr = bind.to_owned();
        std::thread::spawn(move || {
            let r = r_to;
            let s = s_from;

            let s_info = s_info;
            let mut info = NetServerInfo {
                bind_state: BindState::Unknown,
                number_of_client_connected: 0,
            };

            let mut net_streams = Vec::new();
            //Thread that will give us the connected clients
            let (s_bind_state, r_bind_state) = unbounded::<BindState>();
            let (s_of_net_stream, r_of_net_stream) = unbounded::<NetStream>();
            std::thread::spawn(move || match TcpListener::bind(bind_addr) {
                Ok(listener) => {
                    s_bind_state.send(BindState::Success).unwrap();
                    for stream in listener.incoming() {
                        let stream = stream.unwrap();
                        log::info!("Connection established!");
                        let net_stream = NetStream::new(stream);
                        s_of_net_stream.try_send(net_stream).unwrap();
                    }
                }
                _ => {
                    s_bind_state.send(BindState::Error).unwrap();
                }
            });

            let mut loop_helper = LoopHelper::builder().build_with_target_rate(100.0_f64);
            loop {
                if info.bind_state == BindState::Unknown {
                    if let Some(bind_state) = r_bind_state.try_iter().last() {
                        info.bind_state = bind_state;
                    }
                }

                loop_helper.loop_sleep();
                loop_helper.loop_start();
                let net_streams = &mut net_streams;
                match r_of_net_stream.try_recv() {
                    Ok(net_stream) => {
                        log::info!("Connection taken care of");
                        net_streams.push(net_stream);
                    }
                    _ => {}
                }

                //Block on waiting new frames
                match r.try_recv() {
                    Ok(ToNetServerInner::DataToComputeNextFrame(data)) => {
                        let bytes = bincode::serialize(&data).unwrap();
                        for net_stream in net_streams.iter_mut() {
                            net_stream.send_data_to_compute_next_frame(bytes.clone())
                        }
                    }
                    _ => {}
                }

                //Player input
                let mut player_inputs = Vec::new();
                for net_stream in net_streams.iter_mut() {
                    player_inputs.extend(net_stream.collect_remote_player_input());
                }

                let _ = s.try_send(FromNetServerInner::PlayerInputs(player_inputs));

                //Info update
                info.number_of_client_connected = net_streams.len();
                s_info.try_send(info);
            }
        });
        NetServer {
            s_inner: s_to,
            r_inner: r_from,
            info: NetServerInfo {
                bind_state: BindState::Unknown,
                number_of_client_connected: 0,
            },
            r_info,
        }
    }
    pub fn kill(&mut self) {}

    pub fn collect_remote_players_inputs(&mut self) -> Vec<FrameEventFromPlayer> {
        let mut pis = Vec::new();
        for msg in self.r_inner.try_iter() {
            match msg {
                FromNetServerInner::PlayerInputs(player_inputs) => pis.extend(player_inputs),
            }
        }
        pis
    }

    pub fn broadcast_data_to_compute_next_frame(&mut self, data: DataToComputeNextFrame) {
        let _ = self
            .s_inner
            .try_send(ToNetServerInner::DataToComputeNextFrame(data));
    }

    pub fn get_info(&mut self) -> NetServerInfo {
        let last = self.r_info.try_iter().last();
        if let Some(info) = last {
            self.info = info;
        }
        self.info
    }
}

enum ToNetStream {
    DataToComputeNextFrame(Vec<u8>),
}

enum FromNetStream {
    PlayerInput(Vec<FrameEventFromPlayer>),
}

struct NetStream {
    r: Receiver<FromNetStream>,
    s: Sender<ToNetStream>,
}

impl NetStream {
    fn new(stream: TcpStream) -> Self {
        let (s_to, r_to) = unbounded::<ToNetStream>();
        let (s_from, r_from) = unbounded::<FromNetStream>();

        std::thread::spawn(move || {
            let mut stream = stream;
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2)));
            let _ = stream.set_nodelay(true);
            let r = r_to;
            let s = s_from;
            let mut loop_helper = LoopHelper::builder().build_with_target_rate(100.0_f64);
            loop {
                loop_helper.loop_sleep();
                loop_helper.loop_start();
                log::trace!("read");
                let result_bincode: bincode::Result<Vec<FrameEventFromPlayer>> =
                    bincode::deserialize_from(&mut stream);
                match result_bincode {
                    Ok(player_inputs) => {
                        log::trace!(
                            "   Receive player_inputs ({}) from remote client",
                            player_inputs.len()
                        );
                        let _ = s.try_send(FromNetStream::PlayerInput(player_inputs));
                    }
                    x => {
                        log::trace!("   Error read {:?}", x);
                    }
                }

                //Send last frame to remote player
                if !r.is_empty() {
                    match r.try_iter().last().unwrap() {
                        ToNetStream::DataToComputeNextFrame(data) => {
                            log::debug!("Send frame to remote player ({} bytes)", data.len());
                            let _ = stream.write_all(&data).unwrap();
                            stream.flush().unwrap();
                        }
                    }
                }
            }
        });
        NetStream { s: s_to, r: r_from }
    }

    pub fn collect_remote_player_input(&mut self) -> Vec<FrameEventFromPlayer> {
        let mut pis = Vec::new();
        for msg in self.r.try_iter() {
            match msg {
                FromNetStream::PlayerInput(player_inputs) => pis.extend(player_inputs),
            }
        }
        pis
    }
    pub fn send_data_to_compute_next_frame(&mut self, data: Vec<u8>) {
        let _ = self.s.try_send(ToNetStream::DataToComputeNextFrame(data));
    }
}
