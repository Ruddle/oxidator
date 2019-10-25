use crate::client;
use crate::frame;
use crate::frame_server;
use crate::net_server;
use crate::ToClient;
use crossbeam_channel::{Receiver, Sender};
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
            let mut server: Option<NetServer> = None;
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
                log::trace!("Root manager sleep");
                loop_helper.loop_sleep();
                loop_helper.loop_start();
                log::trace!("Root manager receive");
                //Receiving new frame
                let frame = match r_from_frame_server.recv() {
                    Ok(frame_server::FromFrameServer::NewFrame(new_frame)) => new_frame,
                    _ => panic!("frame_server disconnected"),
                };

                //Receiving local player event
                let (start_server_opt, mut player_inputs) =
                    Self::collect_local_player_inputs(&r_from_client);

                if let Some(start_server) = start_server_opt {
                    server = Some(NetServer::new(&start_server.bind));
                }

                //Extend with remote players
                if let Some(server) = &mut server {
                    player_inputs.extend(server.collect_remote_players_inputs());
                }

                //Frame is now complete and ready to be sent
                let data_to_compute_next_frame = frame::DataToComputeNextFrame {
                    old_frame: frame.clone(),
                    events: player_inputs,
                };

                //Broadcast to remotes
                if let Some(server) = &mut server {
                    server.broadcast_data_to_compute_next_frame(data_to_compute_next_frame.clone());
                }

                //Sending to local frame_server and local client
                let _ = s_to_frame_server.send(
                    frame_server::ToFrameServer::DataToComputeNextFrame(data_to_compute_next_frame),
                );
                let _ = s_to_client_from_root_manager.send(ToClient::NewFrame(frame));
            }
        });
    }

    fn collect_local_player_inputs(
        r_from_client: &Receiver<client::FromClient>,
    ) -> (Option<client::StartServer>, Vec<frame::FrameEvent>) {
        //Receiving player event
        let client_events: Vec<_> = r_from_client.try_iter().collect();

        //Collect multiplayer start
        let start_server_opt = client_events
            .iter()
            .flat_map(|e| match e {
                client::FromClient::StartServer(client::StartServer { bind }) => {
                    Some(client::StartServer { bind: bind.clone() })
                }
                _ => None,
            })
            .next();

        //Collect player input from local player
        let player_inputs: Vec<frame::FrameEvent> = client_events
            .iter()
            .flat_map(|e| match e {
                client::FromClient::PlayerInput(event) => Some(event.clone()),
                _ => None,
            })
            .collect();

        (start_server_opt, player_inputs)
    }
}
