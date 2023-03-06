#![deny(rust_2018_idioms)]

use macroquad::prelude::*;
use state::Event;

mod assets;
mod scripts;
mod state;

fn conf() -> Conf {
    Conf {
        window_title: "Iji.rs".to_string(),
        window_width: 800,
        window_height: 600,
        ..Default::default()
    }
}

fn main() {
    let content = gmk_file::parse("ref/source code/iji.gmk");

    macroquad::Window::from_config(conf(), run_main(content))
}

async fn run_main(content: gmk_file::Content) {
    let global = state::Global::new(content);

    global.goto_room_order(0);

    loop {
        for &key in state::KEY_CODES {
            if is_key_pressed(key) {
                global.dispatch(Event::KeyPress(key));
            }
            if is_key_down(key) {
                global.dispatch(Event::KeyDown(key));
            }
            if is_key_released(key) {
                global.dispatch(Event::KeyRelease(key));
            }
        }
        global.step();

        if is_key_pressed(KeyCode::F11) {
            global.dump();
        }

        global.draw();

        next_frame().await;

        global.cleanup();
    }
}
