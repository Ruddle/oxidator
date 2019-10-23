use crate::frame::*;
use crate::group_behavior;
use std::time::Instant;

pub enum ToFrameServer {
    AskNextFrameMsg { old_frame: Frame },
}

pub enum FromFrameServer {
    NewFrame(Frame),
}

pub fn next_frame(mut old_frame: Frame) -> Frame {
    let mut frame_profiler = FrameProfiler::new();
    let start = std::time::Instant::now();
    log::debug!("Received frame {} to compute next frame", old_frame.number);

    log::debug!("Event {}", old_frame.events.len());

    let mut replacer = None;
    for event in old_frame.events.iter() {
        match event {
            FrameEvent::ReplaceFrame(frame) => {
                replacer = Some(frame.clone());
                log::debug!("Replacing frame");
            }
            _ => {}
        }
    }

    let mut frame = replacer.unwrap_or(old_frame);
    frame.number += 1;

    for event in frame.events {
        match event {
            FrameEvent::PlayerInput {
                id,
                input_state,
                selected,
                mouse_world_pos,
            } => {
                group_behavior::Group::update_mobile_target(
                    &input_state.mouse_trigger,
                    mouse_world_pos,
                    &selected,
                    &mut frame.kbots,
                );
            }
            _ => {}
        }
    }

    frame_profiler.add("handle_events", start.elapsed());

    let mut arrows = Vec::new();

    let start_update_units = Instant::now();

    let profiles = group_behavior::Group::update_units(
        &mut frame_profiler,
        &mut frame.kbots,
        &mut frame.kinematic_projectiles,
        &frame.heightmap_phy,
        &mut arrows,
        frame.number,
        &frame.players,
    );

    frame_profiler.add("0 update_units", start_update_units.elapsed());
    frame_profiler.add("total", start.elapsed());
    Frame {
        number: frame.number,
        events: Vec::new(),
        complete: false,
        frame_profiler,
        arrows,
        ..frame
    }
}
