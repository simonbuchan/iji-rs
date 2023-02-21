#![deny(rust_2018_idioms)]

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

use macroquad::prelude::*;

use gml::eval::Function;

fn conf() -> Conf {
    Conf {
        window_title: "Iji.rs".to_string(),
        window_width: 800,
        window_height: 600,
        ..Default::default()
    }
}

fn color_u32(value: u32) -> Color {
    let [a, r, g, b] = value.to_be_bytes();
    Color::from_rgba(r, g, b, a)
}

fn clear_background_u32(color: u32) {
    clear_background(color_u32(color));
}

// struct Room {
//     background: Color,
//     layers: Vec<Layer>,
// }
//
// struct Layer {
//     enabled: bool,
//     texture: Texture2D,
//     pos: Vec2,
//     source: Option<Rect>,
//     tile: bool,
// }
//
// impl Layer {
//     fn draw(&self, view: View) {
//         let pos = self.pos - view.offset;
//         if !self.tile {
//             draw_texture_ex(self.texture, self.pos.x)
//         }
//     }
// }
//
// struct View {
//     screen_rect: Rect,
//     offset: Vec2,
// }

fn write_debug_logs(dir: &str, chunk: &gmk_file::ResourceChunk<impl std::fmt::Debug>) {
    for (name, item) in chunk {
        let parent = std::path::Path::new("ref/out").join(dir).join(name);
        std::fs::create_dir_all(&parent).unwrap();
        std::fs::write(parent.join("debug.log"), format!("{item:#?}")).unwrap();
    }
}

fn main() {
    let content = gmk_file::parse();
    for (name, res) in &content.scripts {
        println!("{name}:");
        gml::ast::parse(&res.script.0).unwrap();
    }
    return;
    // write_debug_logs("objects", &content.objects);
    // for (name, object) in &content.objects {
    //     let parent = std::path::Path::new("ref/out/objects").join(name);
    //     for (event_id, event) in &object.events {
    //         use gmk_file::EventId;
    //         let event_id = match *event_id {
    //             EventId::Create => "Create".to_string(),
    //             EventId::Destroy => "Destroy".to_string(),
    //             EventId::Alarm(id) => format!("Alarm-{id}"),
    //             EventId::Step(id) => format!("Step-{id:?}"),
    //             EventId::Collision(id) => format!("Collision-{id:?}"),
    //             EventId::Keyboard(id) => format!("Key-{id:?}"),
    //             EventId::Mouse(id) => format!("Mouse-{id:?}"),
    //             EventId::Other(id) => format!("Other-{id:?}"),
    //             EventId::Draw(id) => format!("Draw-{id:?}"),
    //             EventId::KeyPress(id) => format!("KeyPress-{id:?}"),
    //             EventId::KeyRelease(id) => format!("KeyRelease-{id:?}"),
    //             EventId::Trigger(id) => format!("Trigger-{id:?}"),
    //             _ => panic!("unknown event id"),
    //         };
    //         for (i, action) in event.actions.iter().enumerate() {
    //             let path = parent.join(format!("{event_id}-{i}.gml"));
    //             if action.kind == gmk_file::ActionKind::Code {
    //                 let code = &action.argument_values[0].0;
    //                 std::fs::write(path, code);
    //             }
    //         }
    //     }
    // }
    // for (name, background) in &content.backgrounds {
    //     if let Some(data) = background
    //         .image
    //         .as_ref()
    //         .and_then(|image| image.data.as_ref())
    //     {
    //         std::fs::write(
    //             std::path::Path::new("ref/out/backgrounds")
    //                 .join(name)
    //                 .join("image.bmp"),
    //             data,
    //         )
    //         .unwrap();
    //     }
    // }
    // return;
    let room = content.rooms.get("rom_main").unwrap();

    let mut ctx = gml::eval::Context::new();
    ctx.def_fn(
        "file_exists",
        Function::new(|_ctx, args| {
            let path = args.get(0).cloned().unwrap_or_default().to_str();
            println!("file_exists: {path:?}");
            Ok(true.into())
        }),
    );
    ctx.def_fn(
        "sound_stop_all",
        Function::new(|_ctx, _args| {
            println!("sound_stop_all");
            Ok(gml::eval::Value::Undefined)
        }),
    );

    for (name, script) in &content.scripts {
        enum Script {
            Unparsed(String),
            Parsed(gml::ast::Script),
        }

        let script = RefCell::new(Script::Unparsed(script.script.0.clone()));
        ctx.def_fn(
            name,
            Function::new(move |ctx, _args| {
                println!("script {name}");
                let mut ref_mut = script.borrow_mut();
                match &*ref_mut {
                    Script::Unparsed(source) => {
                        let parsed = gml::parse(source).unwrap();
                        let result = ctx.exec_script(&parsed);

                        *ref_mut = Script::Parsed(parsed);

                        result
                    }
                    Script::Parsed(script) => ctx.exec_script(&script),
                }
            }),
        );
    }

    for i in &room.instances {
        let id = ctx.add_object();
        let o = &content.objects[i.object_index];
        if let Some(create) = o.events.get(&gmk_file::EventId::Create) {
            for action in &create.actions {
                if action.kind == gmk_file::ActionKind::Code {
                    let code = &action.argument_values[0].0;
                    println!("instance {} create inline code", i.id);
                    let script = gml::parse(code).unwrap();
                    let result = ctx.with_instance(id, |ctx| ctx.exec_script(&script));
                    println!("{result:?}");
                } else if action.kind == gmk_file::ActionKind::Normal
                    && action.exec == gmk_file::ActionExec::Function
                    && action.function_name.0 == "action_execute_script"
                {
                    let index = action.argument_values[0].0.parse().unwrap();
                    let code = &content.scripts[index].script.0;
                    println!("instance {} create script {index}", i.id);
                    let script = gml::parse(code).unwrap();
                    let result = ctx.with_instance(id, |ctx| ctx.exec_script(&script));
                    println!("{result:?}");
                }
            }
        }
    }

    macroquad::Window::from_config(conf(), run_main(content))
}

async fn run_main(content: gmk_file::Content) {
    let room = content.rooms.get("rom_main").unwrap();

    let mut background_textures = HashMap::new();

    let mut sprite_textures = HashMap::new();

    let mut get_background = |index: u32| {
        let background = &content.backgrounds[index];
        *background_textures.entry(index).or_insert_with(|| {
            let data = background.image.as_ref().unwrap().data.as_ref().unwrap();
            Texture2D::from_file_with_format(data, None)
        })
    };

    loop {
        let frame = (get_time() * 15.0) as u32;

        clear_background_u32(content.settings.background_color);
        if room.draw_background_color == gmk_file::Bool32::True {
            clear_background_u32(room.background_color);
        }

        let p = &room.views[0].view_pos;

        for b in &room.backgrounds {
            if b.background_image_index < 0 {
                continue;
            }

            let texture = get_background(b.background_image_index as u32);

            draw_texture(texture, b.pos.0 as f32, b.pos.1 as f32, WHITE);
        }

        for t in &room.tiles {
            let texture = get_background(t.background_index);
            draw_texture_ex(
                texture,
                t.pos.0 as f32,
                t.pos.1 as f32,
                WHITE,
                DrawTextureParams {
                    source: Some(Rect {
                        x: t.tile.0 as f32,
                        y: t.tile.1 as f32,
                        w: t.size.0 as f32,
                        h: t.size.1 as f32,
                    }),
                    ..Default::default()
                },
            );
        }

        for i in &room.instances {
            let obj = &content.objects[i.object_index];
            if obj.visible == gmk_file::Bool32::False || obj.sprite_index < 0 {
                continue;
            }
            let sprite = &content.sprites[obj.sprite_index as u32];

            let textures = sprite_textures.entry(obj.sprite_index).or_insert_with(|| {
                sprite
                    .subimages
                    .iter()
                    .map(|image| {
                        let data = image.data.as_ref().unwrap();
                        let mut image = Image::from_file_with_format(data, None);

                        if sprite.transparent == gmk_file::Bool32::True {
                            let data = image.get_image_data_mut();
                            let t = data[0];
                            for p in data {
                                if *p == t {
                                    *p = [0, 0, 0, 0];
                                }
                            }
                        }

                        Texture2D::from_image(&image)
                    })
                    .collect::<Vec<_>>()
            });

            let texture = textures[frame as usize % textures.len()];
            draw_texture(
                texture,
                (i.pos.0 + sprite.origin.0 as i32) as f32,
                (i.pos.1 + sprite.origin.1 as i32) as f32,
                WHITE,
            );
        }

        next_frame().await;
    }
}

struct TextureSet<K, F> {
    map: HashMap<K, Texture2D>,
    load: F,
}

impl<K: Copy + Eq + std::hash::Hash, F: FnMut(K) -> Texture2D> TextureSet<K, F> {
    pub fn new(load: F) -> Self {
        Self {
            map: Default::default(),
            load,
        }
    }

    pub fn get(&mut self, index: K) -> Texture2D {
        *self.map.entry(index).or_insert_with(|| (self.load)(index))
    }
}

fn discover_fns(content: &gmk_file::Content) {
    #[derive(Debug, Default)]
    struct Visitor {
        fn_defs: BTreeSet<gml::String>,
        fn_refs: BTreeSet<gml::String>,
    }

    let mut visitor = Visitor::default();

    for (id, source) in enum_scripts(&content) {
        if let ScriptId::Resource(name) = id {
            visitor.fn_defs.insert(name.into());
        }
        let file = gml::parse(source).unwrap();
        file.visit(&mut visitor);
    }

    for undef in visitor.fn_refs.difference(&visitor.fn_defs) {
        println!("- {undef}");
    }

    impl gml::ast::Visitor for Visitor {
        fn expr(&mut self, value: &gml::ast::Expr) -> bool {
            if let gml::ast::Expr::Call { id, .. } = value {
                self.fn_refs.insert(id.clone());
            }
            true
        }
    }
}

fn enum_scripts(content: &gmk_file::Content) -> impl Iterator<Item = (ScriptId<'_>, &str)> {
    content
        .scripts
        .iter()
        .map(|(name, res)| (ScriptId::Resource(name), res.script.0.as_str()))
}

enum ScriptId<'a> {
    Resource(&'a str),
    RoomInit,
    InstanceInit,
    TimelineAction,
}