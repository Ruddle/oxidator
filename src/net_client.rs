use crate::frame::*;
use crossbeam_channel::{unbounded, Receiver, Sender};
use spin_sleep::LoopHelper;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpListener;
use std::net::TcpStream;

#[derive(Debug, Clone, Copy)]
pub enum BindState {
    Unknown,
    Success,
    Error,
    Disconnected,
}

#[derive(Debug, Clone, Copy)]
pub struct NetClientInfo {
    bind_state: BindState,
}

pub enum ToNetClientInner {
    PlayerInput(Vec<FrameEventFromPlayer>),
}

pub enum FromNetClientInner {
    DataToComputeNextFrame(DataToComputeNextFrame),
}

pub struct NetClient {
    s: Sender<ToNetClientInner>,
    r: Receiver<FromNetClientInner>,
    info: NetClientInfo,
    r_info: Receiver<NetClientInfo>,
    s_kill: Sender<()>,
}

impl NetClient {
    pub fn new(bind: &str) -> Self {
        let (s_to, r_to) = unbounded::<ToNetClientInner>();
        let (s_from, r_from) = unbounded::<FromNetClientInner>();

        let (s_info, r_info) = unbounded::<NetClientInfo>();

        let (s_kill, r_kill) = unbounded::<()>();

        let bind_addr = bind.to_owned();
        std::thread::spawn(move || {
            let s = s_from;
            let r = r_to;
            let s_info = s_info;

            match TcpStream::connect(bind_addr) {
                Ok(mut stream) => {
                    s_info
                        .try_send(NetClientInfo {
                            bind_state: BindState::Success,
                        })
                        .unwrap();

                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2)));
                    let _ = stream.set_nodelay(true);
                    log::info!("Connection established!");

                    let mut loop_helper = LoopHelper::builder().build_with_target_rate(100.0_f64);
                    'streamloop: loop {
                        loop_helper.loop_sleep();
                        loop_helper.loop_start();
                        match r.try_recv() {
                            Ok(ToNetClientInner::PlayerInput(fe)) => {
                                log::trace!("stream: Sending local player input to remote server");
                                bincode::serialize_into(&mut stream, &fe).unwrap();
                            }
                            _ => {
                                log::trace!("no player input to send");
                            }
                        }

                        log::trace!("read");
                        let result_bincode: bincode::Result<DataToComputeNextFrame> =
                            bincode::deserialize_from(&mut stream);
                        match result_bincode {
                            Ok(data) => {
                                log::trace!("   Receive Frame from remote server");
                                let _ =
                                    s.try_send(FromNetClientInner::DataToComputeNextFrame(data));
                            }
                            x => {
                                log::trace!("   Error read {:?}", x);
                            }
                        }

                        if let Ok(()) = r_kill.try_recv() {
                            let _ = s_info.try_send(NetClientInfo {
                                bind_state: BindState::Disconnected,
                            });
                            break 'streamloop;
                        }
                    }
                    log::info!("Killed");
                }
                _ => {
                    s_info
                        .try_send(NetClientInfo {
                            bind_state: BindState::Error,
                        })
                        .unwrap();
                }
            }
        });
        NetClient {
            s: s_to,
            r: r_from,
            r_info,
            info: NetClientInfo {
                bind_state: BindState::Unknown,
            },
            s_kill,
        }
    }

    pub fn kill(&mut self) {
        self.s_kill.try_send(()).unwrap();
    }

    pub fn collect_data_to_compute_next_frame(&mut self) -> Option<DataToComputeNextFrame> {
        if self.r.is_empty() {
            match self.r.recv() {
                Ok(FromNetClientInner::DataToComputeNextFrame(data)) => Some(data),
                _ => None,
            }
        } else {
            match self.r.try_iter().last().unwrap() {
                FromNetClientInner::DataToComputeNextFrame(data) => Some(data),
            }
        }
    }

    pub fn send_player_inputs(&mut self, player_inputs: Vec<FrameEventFromPlayer>) {
        if player_inputs.len() > 0 {
            log::trace!("net_client: Sending local player input to remote server");
            let _ = self
                .s
                .try_send(ToNetClientInner::PlayerInput(player_inputs));
        }
    }

    pub fn get_info(&mut self) -> NetClientInfo {
        let last = self.r_info.try_iter().last();
        if let Some(info) = last {
            self.info = info;
        }
        self.info
    }
}
