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
        "chr" => {
            let value = args[0].to_int();
            if let Some(value) = value.try_into().ok().and_then(char::from_u32) {
                Ok(String::from_iter([value]).into())
            } else {
                Ok(String::new().into())
            }
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
        | "display_set_all"
        | "window_set_fullscreen"
        | "sound_loop"
        | "sound_stop"
        | "sound_stop_all"
        | "keyboard_set_map"
        | "keyboard_unset_map"
        | "screen_redraw" => Ok(().into()),

        "place_meeting" => {
            let x = args[0].to_int();
            let y = args[1].to_int();
            let id = args[2].try_to_object_id()?;

            // todo: `with (all) place_meeting()` etc.
            let context_instance = {
                let room = global.room.borrow();
                let object_instances = room.object_instances.borrow();
                object_instances[context.instance_id.instance_id()].clone()
            };

            let context_state = context_instance.state.borrow();
            let Some(context_sprite) = context_state.sprite_asset else {
                return Ok(false.into());
            };
            let context_bounds = global
                .assets()
                .sprites
                .get(context_sprite)
                .bounds(ivec2(x, y).as_vec2());

            let object_type = global
                .object_types
                .get(&id.instance_id())
                .ok_or_else(|| gml::eval::Error::InvalidObject(id.into()))?;

            for other_instance in object_type.object.instances.borrow().values() {
                let other_state = other_instance.state.borrow();
                let Some(other_sprite) = other_state.sprite_asset else {
                    continue;
                };

                let other_bounds = global
                    .assets()
                    .sprites
                    .get(other_sprite)
                    .bounds(other_state.pos.as_vec2());

                if context_bounds.overlaps(&other_bounds) {
                    return Ok(true.into());
                }
            }

            Ok(false.into())
        }

        "place_free" => {
            let _x = args[0].to_int();
            let _y = args[1].to_int();
            Ok(true.into())
        }

        "room_goto" => {
            let index = args[0].to_int().try_into().expect("invalid room index");
            global.goto_room(index);
            Ok(().into())
        }

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

        "draw_sprite" => {
            let sprite_index = args[0].to_int();
            let image_index = args[1].to_int();
            let x = args[2].to_int();
            let y = args[3].to_int();

            let image_index = usize::try_from(image_index).ok().unwrap_or_else(|| {
                context
                    .instance
                    .member("image_index")
                    .ok()
                    .flatten()
                    .unwrap_or_default()
                    .to_int() as usize
            });

            let sprite = global.loader().get_sprite(sprite_index as u32);
            let assets = global.assets();
            let sprite = assets.sprites.get(sprite);

            draw_texture(sprite.textures[image_index], x as f32, y as f32, WHITE);

            Ok(().into())
        }
        "draw_sprite_stretched_ext" => {
            let sprite_index = args[0].to_int();
            let image_index = args[1].to_int();
            let x = args[2].to_int();
            let y = args[3].to_int();
            let w = args[4].to_int();
            let h = args[5].to_int();
            let [_, r, g, b] = (args[6].to_int() as u32).to_be_bytes();
            let alpha = args[7].to_float();

            let mut color = Color::from_rgba(r, g, b, 255);
            color.a = alpha as f32;

            let image_index = usize::try_from(image_index).ok().unwrap_or_else(|| {
                context
                    .instance
                    .member("image_index")
                    .ok()
                    .flatten()
                    .unwrap_or_default()
                    .to_int() as usize
            });

            let sprite = global.loader().get_sprite(sprite_index as u32);
            let assets = global.assets();
            let sprite = assets.sprites.get(sprite);

            let pos = ivec2(x, y).as_vec2();
            let size = ivec2(w, h).as_vec2();
            draw_texture_ex(
                sprite.textures[image_index],
                pos.x,
                pos.y,
                color,
                DrawTextureParams {
                    dest_size: Some(size),
                    ..Default::default()
                },
            );

            Ok(().into())
        }

        "draw_set_blend_mode" => Ok(().into()),

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
            instance.dispatch(global, Event::Create);

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

        "game_end" => {
            std::process::exit(0);
        }

        _ => Err(gml::eval::Error::UndefinedFunction(id.to_string())),
    }
}
