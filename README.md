# What is this?

A real time strategy game/engine written with Rust and WebGPU.
Eventually it will be able to run in a web browser thanks to WebGPU.
This project is inspired by Total Annihilation, Supreme Commander, [Spring Engine](https://springrts.com/) and [Zero-k](https://zero-k.info/).

## Demo

![Map editor](etc/map_editor.gif)
Map editor [HQ version](https://streamable.com/e5zdj)

![Unit editor](etc/unit_editor.gif)
Unit editor [HQ version](https://streamable.com/ywr44)

![Play mode](etc/play.gif)
Gameplay (35000 units) [HQ version](https://streamable.com/499j0)



## Goal

The ultimate goal is to provide a modern, carefully crafted, minimal and highly constrained set of tools for players/designers to create mods (game variant) without programming knowledge. 
Those tool would be comprised of :
- Map editor
- Unit editor
    * 3d model import, animation, behavior definition (simple visual programming language, non turing complete), formation ...
- Mod(set of units) editor
- Online repository to publish maps, units, and mods.
- Multiplayer lobby and client/server system
    * Where player can select a tuple of map and mod to play with others
    * Aiming for a quite higher latency than usual (100ms + ping) between server and clients
- High performance multithreaded renderer 
    * Aiming for 100k moving units at 60fps, 1080p with middle-end 2016 hardware
- High performance multithreaded simulation
    * same goal than the renderer

All in one executable.

the test for this goal would be to be able implement a [Zero-k](https://zero-k.info/) clone in this project, with all its features and play it with 32 players.

## Non-Goals

* General purpose modding/ turing complete scripting language.

* Low latency server: the increase in dev time between 10ms latency and 100ms is huge. Not worth the effort given that I want people from all around the world to play together. Also I want games to be about strategy, not action per minute (I am getting too old for this ^^).

* Hosted simulation server. I am broke, people will have to host their own server to play with others (that would just mean clicking the host button in the multi lobby, and having sufficient bandwidth). the online repository will help with discovery though.  

* Deterministic engine: that makes multithreading less efficient and harder. It has nice properties (low file size for replay, debugging) but for my goals they are not worth their price. It is usually done to make networking easier and extremely low bandwidth (because peers only have to share player inputs). I have a plan to keep 80% of those properties with a non-deterministic engine (for 20% the price in dev time). 

* Replacing [Spring Engine](https://springrts.com/): This engine will be far more constrained than spring for performance and time reason. There will not be a scripting language like Lua to mod. However I will make sure everything that has been done in the most popular mods of spring will be doable here. 

## Features

- Map editor
    - [x] raise, lower, flatten, blur, noise pencil
    - [ ] texture layer
    - [ ] changeable map size/water level
    - [ ] resources placing
    - [ ] save and load from file system (try the current save, load button at your own risk)
    - [ ] save and load from online repository  

- Unit editor
    - [x] basic imgui editor to create node hierarchy
    - [x] joint selection (Fix, weapon aim, wheel)
    - [x] mesh selection
    - [x] parameter editing (speed, turn rate, health...)
    - [ ] save/load from filesystem
    - [ ] graphical editor
    - [ ] integration with online repository

- Mod editor
    - [ ] N/A

- Online repository
    - [ ] N/A

- Multiplayer
    - [x] working PoC localhost tcp client/server (1/2 will fry your computer and consume 1 Mo/s) 
    - [ ] optimise to reach 300 Ko/sec with 100k units moving
    - [ ] lobby
    - [ ] live swapping host if current host disconnect
    - [ ] simple chat
    - [ ] ability to draw on the map, and tag place/units
- Rendering 
    - [x] basic display of 3D models (with instancing)
    - [x] basic display a heightmap (from [this blog](http://casual-effects.blogspot.com/2014/04/fast-terrain-rendering-with-continuous.html)) 
    - [x] fxaa (from [this blog](http://blog.simonrodriguez.fr/articles/30-07-2016_implementing_fxaa.html))
    - [x] screen space reflection for water
    - [ ] materials
    - [ ] particles
    - [ ] sounds
    - [ ] animation system
    - [ ] inverted kinematics

- Simulation 
    - [x] working draft of collision detection
    - [x] working draft of flock behavior
    - [x] basic health and damage computation
    - [x] construction and repair
    - [ ] detection (visual and radar)
    - [ ] user-defined AI for units (follow target, formation, flee, target selection etc)
    - [ ] resource counting
    - [ ] integrating pathfinding (I already built a working flowfield pathfinding [here](https://github.com/Ruddle/rustfield))
    
- UI
    - [x] select units (picking and rectangle selection)
    - [x] give move order
    - [x] give build order
    - [x] display current order (Hold LShift)
    - [ ] give user defined, unit specific order
    - [ ] display info about game state (current resources etc)
    - [ ] display info about selected units
    - [ ] display statistics


## Supported platforms

 * Windows (dx12 and vulkan)
 * Linux (vulkan)
 * Mac Untested (*should just work TM*)

All thanks to WebGPU and [wgpu-rs](https://github.com/gfx-rs/wgpu-rs) (and [winit](https://github.com/rust-windowing/winit)). No efforts was made by me in this regard. If anything I posted a stupid issue there that was solved in 0.03 second.
 
## Build

```text
git clone https://github.com/Ruddle/oxidator
cd oxidator
cargo run --release
```

## Fun stuff if you clone this

Shaders are automatically hot-reloaded if you change any .frag or .vert file and you compiled with either shaderc 
```text
cargo run --features use_shaderc --release
```
or glsl_to_spirv
```text
cargo run --features use_glsl_to_spirv --release
```

Logs can be enabled via the [environment variable RUST_LOG](https://github.com/sebasmagri/env_logger/).
for instance:
```text
RUST_LOG=oxidator=debug cargo run --release
```

My go-to command when I develop:
```text
RUST_LOG=oxidator=debug cargo run --features use_shaderc --release
```

You can also compile all the shaders to spirv without starting the full application. The compiled shaders will be saved in /src/shader/compiled. This folder is used when no shader compiler flag is enabled.

```text
cargo run --features use_shaderc --release compile
```


## Roadmap

I push features that I feel like pushing in the moment. 
This project could (and probably will) lose its only contributor any time before any playable milestone is reached.