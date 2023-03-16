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

    let mut server = debug::Server::start(8000);

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

        server.pump(&global).unwrap();
    }
}

mod debug {
    use crate::state::Global;
    use std::io::Read;
    use std::net::Ipv4Addr;
    use std::{fs, io};
    use tiny_http::{Response, ResponseBox};

    pub struct Server(tiny_http::Server);

    impl Server {
        pub fn start(port: u16) -> Self {
            let server = tiny_http::Server::http((Ipv4Addr::LOCALHOST, port)).unwrap();
            println!("Debug serving on http://localhost:{port}/");
            Self(server)
        }

        pub fn pump(&mut self, global: &Global) -> io::Result<()> {
            while let Some(req) = self.0.try_recv()? {
                match req.url() {
                    "/" => req.respond(index())?,
                    "/state" => req.respond(state(global))?,
                    _ => req.respond(tiny_http::Response::from_string("not found"))?,
                }
            }
            Ok(())
        }
    }

    fn index() -> ResponseBox {
        let Ok(file) = fs::File::open("src/index.html") else {
            return Response::new_empty(404.into()).boxed()
        };
        Response::from_file(file)
            .with_header(tiny_http::Header::from_bytes(*b"Content-Type", *b"text/html").unwrap())
            .boxed()
    }

    fn state(global: &Global) -> Response<impl Read> {
        tiny_http::Response::from_string(serde_json::to_string(global).unwrap()).with_header(
            tiny_http::Header::from_bytes(*b"Content-Type", *b"application/json").unwrap(),
        )
    }
}
