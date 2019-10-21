use crate::frame::*;
use crate::group_behavior;

pub enum ToFrameServer {
    AskNextFrameMsg { old_frame: Frame },
}

pub enum FromFrameServer {
    NewFrame(Frame),
}

pub fn next_frame(old_frame: Frame) -> Frame {
    log::debug!("Received frame {} to compute next frame", old_frame.number);
    let mut players = old_frame.players.clone();
    let mut kbots = old_frame.kbots.clone();
    let mut kinematic_projectiles = old_frame.kinematic_projectiles.clone();

    log::debug!("Event {}", old_frame.events.len());

    for event in old_frame.events {
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
                    &mut kbots,
                );
            }
        }
    }

    // group_behavior::Group::update_units(
    //     0.0,
    //     &mut kbots,
    //     &mut kinematic_projectiles,
    //     heightmap_data,
    // );

    Frame {
        number: old_frame.number + 1,
        players,
        kbots,
        kinematic_projectiles,
        events: Vec::new(),
        complete: false,
    }
}
