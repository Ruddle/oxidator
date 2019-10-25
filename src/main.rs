mod arrow_gpu;
mod camera;
mod client;
mod fake_texels;
mod frame;
mod frame_server;
mod game_state;
mod glsl_compiler;
mod gpu;

mod health_bar;
mod heightmap;
mod heightmap_editor;
mod heightmap_gpu;
mod heightmap_phy;
mod imgui_wgpu;
mod input_state;
mod mobile;
mod model;
mod model_gpu;

mod net_server;
mod post_fx;
mod post_fxaa;
mod trait_gpu;
mod utils;

extern crate byteorder;
extern crate crossbeam_channel;
extern crate nalgebra as na;
#[cfg(feature = "use_shaderc")]
extern crate shaderc;
#[macro_use]
extern crate typename;
extern crate base_62;
extern crate spin_sleep;
use crossbeam_channel::unbounded;
use spin_sleep::LoopHelper;
use winit::event::Event;
use winit::event_loop::ControlFlow;
#[derive(Debug)]
pub enum ToClient {
    MapReadAsyncMessage { vec: Vec<f32>, usage: String },
    NewFrame(frame::Frame),
}

pub enum EventLoopMsg {
    Stop,
}

fn main() {
    log::info!("dir: {:?}", std::env::current_dir().unwrap());
    env_logger::init();

    let (s_to_client, r_to_client) = unbounded::<ToClient>();

    let s_to_client_from_root_manager = s_to_client.clone();

    let (s_to_event_loop, r_to_event_loop) = unbounded::<EventLoopMsg>();
    let (s_from_client, r_from_client) = unbounded::<client::FromClient>();
    let (s_to_frame_server, r_to_frame_server) = unbounded::<frame_server::ToFrameServer>();
    let (s_from_frame_server, r_from_frame_server) = unbounded::<frame_server::FromFrameServer>();

    //Frame server
    std::thread::spawn(move || {
        let mut fsc = frame_server::FrameServerCache::new();
        for msg in r_to_frame_server.iter() {
            match msg {
                frame_server::ToFrameServer::DataToComputeNextFrame(
                    frame::DataToComputeNextFrame { old_frame, events },
                ) => {
                    let next_frame = fsc.next_frame(old_frame, events);
                    // let vec = bincode::serialize(&next_frame).unwrap();
                    // log::info!("Frame is {} bytes", vec.len());
                    // let mut vec = Vec::new();
                    // for k in next_frame.kbots.values() {
                    //     vec.push(k.clone());
                    // }
                    // let fu = frame::FrameUpdate { kbots: vec };
                    // let vec = bincode::serialize(&fu).unwrap();
                    // log::info!("FrameUpdate is {} bytes", vec.len());
                    // let dur = utils::time(|| {
                    //     use flate2::write::ZlibEncoder;
                    //     use flate2::Compression;
                    //     use std::io::prelude::*;
                    //     let mut e = ZlibEncoder::new(Vec::new(), Compression::new(1));
                    //     e.write_all(&vec);
                    //     let compressed_bytes = e.finish().unwrap();
                    //     log::info!("Compressed is {} bytes", compressed_bytes.len());
                    // });
                    // log::info!("compression took {:?}", dur);

                    let _ = s_from_frame_server
                        .send(frame_server::FromFrameServer::NewFrame(next_frame));
                }
            }
        }
    });

    //Root manager
    std::thread::spawn(move || {
        use net_server::NetServer;
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

            //Receiving player event
            let client_events: Vec<_> = r_from_client.try_iter().collect();

            //Collect multiplayer start
            if let Some(bind) = client_events
                .iter()
                .flat_map(|e| match e {
                    client::FromClient::StartServer { bind } => Some(bind),
                    _ => None,
                })
                .next()
            {
                server = Some(NetServer::new(bind));
            }

            //Collect player input from local player
            let mut player_inputs: Vec<frame::FrameEvent> = client_events
                .iter()
                .flat_map(|e| match e {
                    client::FromClient::PlayerInput(event) => Some(event.clone()),
                    _ => None,
                })
                .collect();

            //Extend with remote players
            if let Some(server) = &mut server {
                player_inputs.extend(server.collect_remote_players_inputs());
            }

            let data_to_compute_next_frame = frame::DataToComputeNextFrame {
                old_frame: frame.clone(),
                events: player_inputs,
            };

            //Frame is now complete and ready to be sent

            //Broadcast to remotes
            if let Some(server) = &mut server {
                server.broadcast(data_to_compute_next_frame.clone());
            }

            //Sending to local frame_server and local client
            let _ = s_to_frame_server.send(frame_server::ToFrameServer::DataToComputeNextFrame(
                data_to_compute_next_frame,
            ));
            let _ = s_to_client_from_root_manager.send(ToClient::NewFrame(frame));
        }
    });

    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    let mut client = client::App::new(
        window,
        s_to_client,
        r_to_client,
        s_to_event_loop,
        s_from_client,
    );

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { .. } => {
            client.handle_winit_event(&event);
        }
        Event::EventsCleared => match r_to_event_loop.try_recv() {
            Ok(EventLoopMsg::Stop) => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {
                client.receive();
                client.render();
            }
        },
        _ => {}
    });
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct World {
    pub a: String,
    pub b: u8,
}

use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

pub fn client() {
    let w = World {
        a: "Stzefeof".to_owned(),
        b: 244,
    };

    let vec = bincode::serialize(&w).unwrap();

    let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();

    let start = std::time::Instant::now();
    let _ = stream.write_all(&vec); // ignore the Result
                                    // let _ = stream.read(&mut [0; 128]); // ignore this too

    println!("{:?}", vec.last().unwrap());
    println!("took {:?}", start.elapsed());
} // the stream is close

pub fn server() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        println!("Connection established!");

        let start = std::time::Instant::now();
        let mut buffer: Vec<u8> = Vec::new();
        let n = stream.read_to_end(&mut buffer).unwrap();
        println!("The bytes: {:?}", n); //&buffer[..n]
        println!("last {:?}", buffer.last().unwrap());
        println!("took {:?}", start.elapsed());

        let w: World = bincode::deserialize(&buffer[..]).unwrap();
        println!("{:?}", w);
    }
}
