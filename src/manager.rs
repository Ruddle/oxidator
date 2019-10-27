use crate::client;
use crate::frame;
use crate::frame_server;
use crate::net_client;
use crate::net_server;
use crate::ToClient;
use crossbeam_channel::{Receiver, Sender};
use net_client::NetClient;
use net_server::NetServer;
use spin_sleep::LoopHelper;
pub struct Manager {}

impl Manager {
    pub fn new(
        s_to_client_from_root_manager: Sender<crate::ToClient>,
        s_to_frame_server: Sender<frame_server::ToFrameServer>,
        r_from_frame_server: Receiver<frame_server::FromFrameServer>,
        r_from_client: Receiver<client::FromClient>,
    ) -> () {
        std::thread::spawn(move || {
            let mut global_info = GlobalInfo {
                manager: ManagerInfo {
                    loop_time: std::time::Duration::from_millis(0),
                },
                net_client: None,
                net_server: None,
            };
            let mut net: Net = Net::Offline;

            let frame0 = frame::Frame::new();
            let _ = s_to_frame_server.send(frame_server::ToFrameServer::DataToComputeNextFrame(
                frame::DataToComputeNextFrame {
                    old_frame: frame0.clone(),
                    events: Vec::new(),
                },
            ));
            let _ = s_to_client_from_root_manager.send(ToClient::NewFrame(frame0));

            let mut loop_helper = LoopHelper::builder().build_with_target_rate(10.0_f64);
            loop {
                log::trace!("loop sleep");
                loop_helper.loop_sleep();
                global_info.manager.loop_time = loop_helper.loop_start();
                log::trace!("receive");
                //Receiving new frame
                let mut frame = match r_from_frame_server.recv() {
                    Ok(frame_server::FromFrameServer::NewFrame(new_frame)) => new_frame,
                    _ => panic!("frame_server disconnected"),
                };

                //Receiving local player event
                let mut player_inputs = Vec::new();
                for from_client in r_from_client.try_iter() {
                    use client::FromClient;
                    match from_client {
                        FromClient::PlayerInput(event) => player_inputs.push(event),
                        FromClient::StartClient(client::StartClient { bind }) => {
                            net = Net::IsClient(NetClient::new(&bind))
                        }
                        FromClient::StartServer(client::StartServer { bind }) => {
                            net = Net::IsServer(NetServer::new(&bind))
                        }
                        FromClient::DisconnectServer => {
                            if let Net::IsServer(net_server) = &mut net {
                                net_server.kill();
                                global_info.net_server = None;
                                net = Net::Offline;
                            }
                        }
                        FromClient::DisconnectClient => {
                            if let Net::IsClient(net_client) = &mut net {
                                net_client.kill();
                                global_info.net_client = None;
                                net = Net::Offline;
                            }
                        }
                    }
                }

                //If local is client : Send player events
                if let Net::IsClient(net_client) = &mut net {
                    net_client.send_player_inputs(
                        player_inputs
                            .iter()
                            .filter(|e| match e {
                                frame::FrameEventFromPlayer::ReplaceFrame(x) => false,
                                _ => true,
                            })
                            .map(|e| e.clone())
                            .collect(),
                    );
                }
                //If local is server : Extend with remote players
                else if let Net::IsServer(server) = &mut net {
                    player_inputs.extend(server.collect_remote_players_inputs());
                }

                //Frame is now complete and ready to be sent
                let mut data_to_compute_next_frame = frame::DataToComputeNextFrame {
                    old_frame: frame.clone(),
                    events: player_inputs,
                };

                //If local is client : Get remote frame (TEMPORARY TOTAL BYPASS OF LOCAL FRAME_SERVER)
                if let Net::IsClient(net_client) = &mut net {
                    data_to_compute_next_frame =
                        net_client.collect_data_to_compute_next_frame().unwrap();

                    frame = data_to_compute_next_frame.old_frame.clone();
                }
                //If local is server : Broadcast to remotes
                else if let Net::IsServer(server) = &mut net {
                    server.broadcast_data_to_compute_next_frame(data_to_compute_next_frame.clone());
                }

                //Sending to local frame_server and local client
                let _ = s_to_frame_server.send(
                    frame_server::ToFrameServer::DataToComputeNextFrame(data_to_compute_next_frame),
                );
                let _ = s_to_client_from_root_manager.send(ToClient::NewFrame(frame));

                //Gathering and sending GlobalInfo
                if let Net::IsClient(net_client) = &mut net {
                    global_info.net_client = Some(net_client.get_info());
                } else if let Net::IsServer(server) = &mut net {
                    global_info.net_server = Some(server.get_info());
                }
                let _ = s_to_client_from_root_manager.send(ToClient::GlobalInfo(global_info));
            }
        });
    }
}

enum Net {
    Offline,
    IsServer(NetServer),
    IsClient(NetClient),
}

#[derive(Debug, Clone, Copy)]
pub struct ManagerInfo {
    loop_time: std::time::Duration,
}

///Info about all the components of this program
#[derive(Debug, Clone, Copy)]
pub struct GlobalInfo {
    pub manager: ManagerInfo,
    pub net_server: Option<net_server::NetServerInfo>,
    pub net_client: Option<net_client::NetClientInfo>,
}
