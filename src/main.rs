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
    use std::io::Read;
    use std::net::Ipv4Addr;
    use std::str::FromStr;
    use std::{fs, io};

    use tiny_http::{Header, Response, ResponseBox};

    use crate::state::Global;

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
                    "/" => req.respond(file("index.html", b"text/html"))?,
                    "/state" => req.respond(state(global))?,
                    url => {
                        if let Some(index) = url.strip_prefix("/sprite/") {
                            if let Some((sprite_index, image_index)) = index.split_once('/') {
                                if let (Ok(sprite_index), Ok(image_index)) =
                                    (usize::from_str(sprite_index), usize::from_str(image_index))
                                {
                                    let items = &global.content().sprites.items;
                                    if let Some(Some(item)) = items.get(sprite_index) {
                                        if let Some(image) = item.data.subimages.get(image_index) {
                                            if let Some(data) = &image.data {
                                                return req.respond(
                                                    Response::from_data(data.clone())
                                                        .with_header(type_header(b"image/bmp")),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            return req.respond(Response::empty(404));
                        }
                        let res = file(url.strip_prefix('/').unwrap(), {
                            if url.ends_with(".mjs") {
                                b"application/javascript"
                            } else if url.ends_with(".css") {
                                b"text/css"
                            } else {
                                b"application/octet-stream"
                            }
                        });
                        req.respond(res)?
                    }
                }
            }
            Ok(())
        }
    }

    fn file(path: &str, content_type: &[u8]) -> ResponseBox {
        let path = std::path::Path::new("debug-static").join(path);
        let Ok(file) = fs::File::open(path) else {
            return Response::empty(404).boxed();
        };
        Response::from_file(file)
            .with_header(type_header(content_type))
            .boxed()
    }

    fn state(global: &Global) -> Response<impl Read> {
        tiny_http::Response::from_string(serde_json::to_string(global).unwrap())
            .with_header(type_header(b"application/json"))
    }

    fn type_header(value: &[u8]) -> Header {
        Header::from_bytes(*b"Content-Type", value).unwrap()
    }
}
