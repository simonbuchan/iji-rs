#![deny(rust_2018_idioms)]

use macroquad::prelude::*;
use state::Event;

mod assets;
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
        global.draw();

        next_frame().await;

        global.cleanup();
    }
}

mod scripts {
    use macroquad::prelude::*;

    use gml::eval::{Context, Value};

    use crate::state::{key_code, Event, FontAsset, Global};

    pub fn call(
        global: &Global,
        context: &mut Context<'_>,
        id: &str,
        args: Vec<Value>,
    ) -> gml::eval::Result<Value> {
        match id {
            "floor" => Ok(args[0].to_float().floor().into()),
            "random" => {
                let range = args[0].to_float();
                Ok(rand::gen_range(0.0, range).into())
            }
            "ord" => {
                let value = args[0].to_str();
                let char = value.chars().next();
                Ok(char.map_or(().into(), |char| (char as i32).into()))
            }
            "string" => Ok(args[0].to_str().into()),
            "string_length" => Ok(i32::try_from(args[0].to_str().len())
                .expect("string too long")
                .into()),
            "string_char_at" => {
                let value = args[0].to_str();
                let index = args[1].to_int();
                let char = value.get(index as usize..).and_then(|s| s.chars().next());
                Ok(char.map_or(().into(), |char| (char as i32).into()))
            }

            "file_exists" => {
                let _path = args[0].to_str();
                Ok(false.into())
            }
            "file_text_open_write"
            | "file_text_close"
            | "file_text_write_string"
            | "file_text_writeln"
            | "draw_sprite"
            | "draw_set_blend_mode"
            | "sound_loop"
            | "sound_stop"
            | "sound_stop_all"
            | "keyboard_unset_map" => Ok(().into()),

            "room_goto_next" => {
                global.room_goto_next();
                Ok(().into())
            }

            "keyboard_check" => {
                let key = args[0].to_int();
                if let Ok(key) = key.try_into() {
                    Ok(is_key_pressed(key_code(key)).into())
                } else {
                    Ok(false.into())
                }
            }

            "font_add_sprite" => {
                let sprite_index = args[0].to_int();
                let first = args[1].to_int();
                let _proportional = args[2].to_bool();
                let _sep = args[3].to_bool();

                let id = global
                    .loader()
                    .get_sprite(sprite_index.try_into().expect("invalid sprite index"));
                let id = global.state.borrow_mut().fonts.add(FontAsset::new(
                    id,
                    first.try_into().expect("invalid font char"),
                ));
                Ok(id.into())
            }

            "make_color_rgb" => {
                let r = args[0].to_int();
                let g = args[1].to_int();
                let b = args[2].to_int();
                let color = r & 0xFF | ((g & 0xFF) << 8) | ((b & 0xFF) << 16);
                Ok(color.into())
            }

            "draw_set_font" => {
                let font_index = args[0].to_int();
                global.state.borrow_mut().fonts.set(font_index);
                Ok(().into())
            }

            "draw_text_ext" => {
                let x = args[0].to_int();
                let y = args[1].to_int();
                let string = args[2].to_str();
                let sep = args[3].to_int();
                let w = args[4].to_int();
                let state = global.state.borrow();
                if let Some(font) = state.fonts.get() {
                    font.draw_text(global, ivec2(x, y), &string, sep, w);
                }
                Ok(().into())
            }

            "draw_set_color" => {
                let value = args.get(0).map_or(0, Value::to_int);
                let [r, g, b, _] = value.to_le_bytes();
                global.state.borrow_mut().color = Color::from_rgba(r, g, b, 0xFF);
                Ok(().into())
            }
            "draw_rectangle" => {
                let x1 = args.get(0).map_or(0, Value::to_int);
                let y1 = args.get(1).map_or(0, Value::to_int);
                let x2 = args.get(2).map_or(0, Value::to_int);
                let y2 = args.get(3).map_or(0, Value::to_int);
                let outline = args.get(4).map_or(false, Value::to_bool);
                let pos = ivec2(x1, y1).as_vec2();
                let size = ivec2(x2, y2).as_vec2() - pos;
                let color = global.state.borrow().color;
                if outline {
                    draw_rectangle_lines(pos.x, pos.y, size.x, size.y, 1.0, color);
                } else {
                    draw_rectangle(pos.x, pos.y, size.x, size.y, color);
                }
                Ok(().into())
            }

            "instance_create" => {
                let x = args[0].to_int();
                let y = args[1].to_int();
                let object_index = args[2]
                    .as_int()
                    .and_then(|index| index.try_into().ok())
                    .ok_or_else(|| gml::eval::Error::InvalidInt(args[2].clone()))?;

                let id = global.next_instance_id();
                let id = gml::eval::ObjectId::new(id);
                let instance = global.instance_create(id, ivec2(x, y), object_index);

                instance.dispatch(global, context, Event::Create);

                Ok(id.into())
            }

            "instance_destroy" => {
                let id = args
                    .get(0)
                    .unwrap_or(&context.instance_id.into())
                    .as_object_id()
                    .ok_or_else(|| gml::eval::Error::InvalidObject(args[0].clone()))?;
                global.destroy_instance(id);
                Ok(().into())
            }

            "instance_number" => {
                let object_index = args[0].to_int();
                if let Ok(object_index) = u32::try_from(object_index) {
                    Ok(global.instance_number(object_index).into())
                } else {
                    Ok(0.into())
                }
            }

            _ => Err(gml::eval::Error::UndefinedFunction(id.to_string())),
        }
    }
}
