#[macro_use]
extern crate bitfield;

extern crate rand;
extern crate time;

use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use sdl2::audio;
use sdl2::audio::AudioCallback;
use sdl2::audio::AudioQueue;
use sdl2::audio::AudioSpecDesired;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::libc::SEEK_CUR;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::EventPump;

mod apu;
mod bus;
mod cartridge;
mod controller;
mod cpu;
mod cpu_debug;
mod ppu;

use bus::Bus;
use controller::Button;
use cpu::Cpu;
use sdl2::render::TextureCreator;
use sdl2::video::Window;
use sdl2::video::WindowContext;

struct NesCore<'a> {
    cpu: Cpu,
    frame_count: u64,
    frame_second: u64,
    event_pump: EventPump,
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    audio_device: AudioQueue<i16>
}



impl<'a> NesCore<'_> {
    fn new(event_pump: EventPump, canvas: Canvas<Window>, texture: Texture<'_>, audio_device: AudioQueue<i16>) -> NesCore {
        NesCore {
            cpu: Cpu::new(Bus::new()),
            frame_count: 0,
            frame_second: 0,
            event_pump,
            canvas,
            texture,
            audio_device
        }
    }

    fn load_game(&mut self, game_data: &[u8]) {
        self.cpu.bus.load_rom_from_memory(game_data);
        self.cpu.reset();
        self.cpu.bus.reset();
    }

    fn run(&mut self) {
        self.handle_user_input();

        let second = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("We're going backwards!")
            .as_secs();
        if self.frame_second != second {
            self.frame_count = 0;
            self.frame_second = second;
        }

        while !self.cpu.bus.draw {
            self.cpu.execute_next_instruction();
            let stall_cycles = self.cpu.bus.reset_cpu_stall_cycles();
            for _ in 0..stall_cycles {
                self.cpu.bus.tick()
            }
        }

        self.cpu.bus.draw = false;

        let mut video_frame = [0; 256 * 240 * 4];

        for i in 0..video_frame.len() {
            let pixel = self.cpu.bus.ppu.renderer.pixels[i / 4];
            video_frame[i] = (pixel >> (i % 4 * 8)) as u8;
        }

        let _ = self.texture.update(None, &video_frame, 256 * 4).unwrap();
        self.canvas.copy(&self.texture, None, None).unwrap();
        self.canvas.present();

        let audio_buffer_size = self.cpu.bus.apu.buffer.len();
        if audio_buffer_size < 1470 {
            for _ in 0..1470 - audio_buffer_size {
                self.cpu.bus.apu.buffer.push(0);
            }
        }
        self.audio_device.queue(&self.cpu.bus.apu.buffer[..]);
        // self.audio_device.resume();
        self.cpu.bus.apu.buffer.clear();

        self.frame_count += 1;
    }

    fn handle_user_input(&mut self) {
        use Button::*;
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    std::process::exit(0);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(UP, true);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(DOWN, true);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(LEFT, true);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(RIGHT, true);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::K),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(A, true);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::L),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(B, true);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::N),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(START, true);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::M),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(SELECT, true);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::W),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(UP, false);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::S),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(DOWN, false);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(LEFT, false);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::D),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(RIGHT, false);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::K),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(A, false);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::L),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(B, false);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::N),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(START, false);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::M),
                    ..
                } => {
                    self.cpu.bus.controller_0.set_button_state(SELECT, false);
                }
                _ => {}
            }
        }
    }
}

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("NES Emulator", (32 * 20) as u32, (32 * 20) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.set_scale(2.0, 2.0).unwrap();

    let binding = canvas.texture_creator();
    let texture = binding.create_texture_target(PixelFormatEnum::ARGB8888, 256, 240).unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_audio_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None
    };
    let audio_device = audio_subsystem.open_queue::<i16, _>(None, &desired_audio_spec).unwrap();
    audio_device.resume();

    let mut nes_core = NesCore::new(sdl_context.event_pump().unwrap(), canvas, texture, audio_device);

    let bytes: Vec<u8> = std::fs::read("games/Super Mario Bros. (World).nes").unwrap();

    nes_core.load_game(&bytes);

    loop {
        nes_core.run();
    }
}
