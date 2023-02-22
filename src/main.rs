#![deny(rust_2018_idioms)]

use std::collections::{BTreeSet, HashMap, HashSet};
use std::marker::PhantomData;

use macroquad::prelude::*;
use rayon::prelude::*;

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

struct AssetId<T>(u32, PhantomData<T>);
impl<T> Clone for AssetId<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}
impl<T> Copy for AssetId<T> {}

struct AssetSet<T> {
    items: HashMap<u32, T>,
}

impl<T> Default for AssetSet<T> {
    fn default() -> Self {
        Self {
            items: Default::default(),
        }
    }
}

impl<T> AssetSet<T> {
    fn load_with(&mut self, index: u32, load: impl FnOnce() -> T) -> AssetId<T> {
        self.items.entry(index).or_insert_with(load);
        AssetId(index, PhantomData)
    }

    fn get(&self, id: AssetId<T>) -> &T {
        &self.items[&id.0]
    }
}

#[derive(Default)]
struct Assets {
    backgrounds: AssetSet<BackgroundAsset>,
    sprites: AssetSet<SpriteAsset>,
}

struct Loader<'content> {
    content: &'content gmk_file::Content,
    assets: Assets,
}

impl<'content> Loader<'content> {
    fn new(content: &'content gmk_file::Content) -> Self {
        Self {
            content,
            assets: Default::default(),
        }
    }
}

impl Loader<'_> {
    fn get_background(&mut self, index: u32) -> AssetId<BackgroundAsset> {
        self.assets.backgrounds.load_with(index, || {
            BackgroundAsset::load(&self.content.backgrounds[index])
        })
    }

    fn get_sprite(&mut self, index: u32) -> AssetId<SpriteAsset> {
        self.assets
            .sprites
            .load_with(index, || SpriteAsset::load(&self.content.sprites[index]))
    }
}

struct BackgroundAsset {
    texture: Texture2D,
}

impl Drop for BackgroundAsset {
    fn drop(&mut self) {
        self.texture.delete();
    }
}

impl BackgroundAsset {
    fn load(def: &gmk_file::Background) -> Self {
        let data = def.image.as_ref().unwrap().data.as_ref().unwrap();
        let texture = Texture2D::from_file_with_format(data, None);
        // always present since GM 5.x
        // let tiling = def.tiling.as_ref().unwrap();

        // let mut tile_size = None;
        // if tiling.enabled == gmk_file::Bool32::True {
        //     tiling.
        // }
        Self { texture }
    }
}

struct SpriteAsset {
    size: Vec2,
    origin: Vec2,
    textures: Vec<Texture2D>,
}

impl Drop for SpriteAsset {
    fn drop(&mut self) {
        for t in &self.textures {
            t.delete();
        }
    }
}

impl SpriteAsset {
    fn load(def: &gmk_file::Sprite) -> Self {
        let textures = def
            .subimages
            .iter()
            .map(|image| {
                let data = image.data.as_ref().unwrap();
                let mut image = Image::from_file_with_format(data, None);

                if def.transparent == gmk_file::Bool32::True {
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
            .collect::<Vec<_>>();

        let size = Vec2::new(def.size.0 as f32, def.size.1 as f32);
        let origin = Vec2::new(def.origin.0 as f32, def.origin.1 as f32);

        Self {
            size,
            origin,
            textures,
        }
    }
}

struct Room {
    assets: Assets,
    view: View,
    background_color: Color,
    background_layers: Vec<Layer>,
    tiles: Vec<Tile>,
    instances: Vec<Instance>,
    foreground_layers: Vec<Layer>,
}

impl Room {
    pub fn load(loader: &mut Loader<'_>, ctx: &mut gml::eval::Context, name: &str) -> Self {
        loader.assets = Assets::default();

        let def = loader.content.rooms.get(name).unwrap();
        let mut background_layers = vec![];
        let mut tiles = vec![];
        let mut instances = vec![];
        let mut foreground_layers = vec![];

        for b in &def.backgrounds {
            let Ok(index) = b.background_image_index.try_into() else {
                continue;
            };

            if b.foreground_image.into() {
                &mut foreground_layers
            } else {
                &mut background_layers
            }
            .push(Layer {
                enabled: b.visible.into(),
                pos: IVec2::new(b.pos.0, b.pos.1).as_vec2(),
                asset: loader.get_background(index),
                tile: false,
                source: None,
            });
        }

        for t in &def.tiles {
            tiles.push(Tile {
                depth: t.depth,
                asset: loader.get_background(t.background_index),
                pos: IVec2::new(t.pos.0, t.pos.1).as_vec2(),
                source: Rect {
                    x: t.tile.0 as f32,
                    y: t.tile.1 as f32,
                    w: t.size.0 as f32,
                    h: t.size.1 as f32,
                },
            });
        }

        for i in &def.instances {
            assert_eq!(&*i.creation_code, "");
            let obj = &loader.content.objects[i.object_index];
            assert!(obj.mask_sprite_index < 0);
            assert!(obj.parent_object_index < 0);
            instances.push(Instance {
                frame: 0,
                visible: obj.visible.into(),
                depth: obj.depth.into(),
                gml_object: ctx.add_object(),
                object_index: i.object_index,
                sprite_asset: obj
                    .sprite_index
                    .try_into()
                    .ok()
                    .map(|index| loader.get_sprite(index)),
                pos: IVec2::new(i.pos.0, i.pos.1).as_vec2(),
            });
        }

        Self {
            assets: std::mem::take(&mut loader.assets),
            view: View {
                offset: Vec2::default(),
                size: Vec2::new(screen_width(), screen_height()),
            },
            background_color: color_u32(def.background_color),
            background_layers,
            tiles,
            instances,
            foreground_layers,
        }
    }

    fn draw(&self) {
        clear_background(self.background_color);
        for layer in &self.background_layers {
            layer.draw(&self.assets, &self.view);
        }
        for tile in &self.tiles {
            tile.draw(&self.assets, &self.view);
        }
        for instance in &self.instances {
            instance.draw(&self.assets, &self.view);
        }
        for layer in &self.background_layers {
            layer.draw(&self.assets, &self.view);
        }
    }
}

struct View {
    offset: Vec2,
    size: Vec2,
}

struct Layer {
    enabled: bool,
    asset: AssetId<BackgroundAsset>,
    pos: Vec2,
    source: Option<Rect>,
    tile: bool,
}

impl Layer {
    fn draw(&self, assets: &Assets, view: &View) {
        if !self.enabled {
            return;
        }

        let pos = self.pos - view.offset;
        if !self.tile {
            draw_texture_ex(
                assets.backgrounds.get(self.asset).texture,
                pos.x,
                pos.y,
                WHITE,
                DrawTextureParams {
                    ..Default::default()
                },
            )
        }
    }
}

struct Tile {
    depth: u32,
    asset: AssetId<BackgroundAsset>,
    pos: Vec2,
    source: Rect,
}

impl Tile {
    fn draw(&self, assets: &Assets, view: &View) {
        let pos = self.pos - view.offset;
        draw_texture_ex(
            assets.backgrounds.get(self.asset).texture,
            pos.x,
            pos.y,
            WHITE,
            DrawTextureParams {
                source: Some(self.source),
                ..Default::default()
            },
        );
    }
}

struct Instance {
    sprite_asset: Option<AssetId<SpriteAsset>>,
    depth: u32,
    visible: bool,
    object_index: u32,
    gml_object: gml::eval::ObjectId,
    pos: Vec2,
    frame: usize,
}

impl Instance {
    fn draw(&self, assets: &Assets, view: &View) {
        let Some(sprite) = self.sprite_asset else {
            return;
        };
        let sprite = assets.sprites.get(sprite);

        let texture = sprite.textures[self.frame];
        let pos = self.pos + sprite.origin - view.offset;
        draw_texture(texture, pos.x, pos.y, WHITE);
    }
}

fn write_debug_logs(dir: &str, chunk: &gmk_file::ResourceChunk<impl std::fmt::Debug>) {
    for (name, item) in chunk {
        let parent = std::path::Path::new("ref/out").join(dir).join(name);
        std::fs::create_dir_all(&parent).unwrap();
        std::fs::write(parent.join("debug.log"), format!("{item:#?}")).unwrap();
    }
}

fn main() {
    let content = gmk_file::parse("ref/source code/iji.gmk");
    // for (name, res) in &content.scripts {
    //     println!("{name}:");
    //     let parent = std::path::Path::new("ref/unpacked/scripts");
    //     std::fs::create_dir_all(parent).unwrap();
    //     std::fs::write(parent.join(format!("{name}.gml")), &res.script.0).unwrap();
    //     gml::parse(&res.script.0).unwrap();
    // }
    // return;
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

    let scripts: Vec<Option<(String, gml::ast::Script)>> = content
        .scripts
        .items
        .par_iter()
        .map(|item| {
            item.as_ref().map(|item| {
                let script = gml::parse(&item.data.script.0).unwrap();
                (item.name.0.clone(), script)
            })
        })
        .collect();

    let mut ctx = gml::eval::Context::new();
    ctx.def_fn("floor", |_ctx, args| Ok(args[0].to_float().floor().into()));
    ctx.def_fn("random", |_ctx, args| {
        let range = args[0].to_float();
        Ok(rand::gen_range(0.0, range).into())
    });

    ctx.def_fn("ord", |_ctx, args| {
        let value = args[0].to_str();
        let char = value.chars().next();
        Ok(char.map_or(().into(), |char| (char as i32).into()))
    });
    ctx.def_fn("string", |_ctx, args| Ok(args[0].to_str().into()));
    ctx.def_fn("string_char_at", |_ctx, args| {
        let value = args[0].to_str();
        let index = args[1].to_int();
        let char = value.get(index as usize..).and_then(|s| s.chars().next());
        Ok(char.map_or(().into(), |char| (char as i32).into()))
    });

    ctx.def_fn("file_exists", |_ctx, args| {
        let _path = args[0].to_str();
        Ok(false.into())
    });
    ctx.def_fn("file_text_open_write", |_ctx, _args| Ok(().into()));
    ctx.def_fn("file_text_close", |_ctx, _args| Ok(().into()));
    ctx.def_fn("file_text_write_string", |_ctx, _args| Ok(().into()));
    ctx.def_fn("file_text_writeln", |_ctx, _args| Ok(().into()));

    ctx.def_fn("sound_stop_all", |_ctx, _args| Ok(().into()));

    ctx.def_fn("keyboard_unset_map", |_ctx, _args| Ok(().into()));

    ctx.def_fn("instance_create", |ctx, args| {
        println!("instance_create({args:?})");
        let id = ctx.add_object();
        // todo: also add to room instances
        Ok(id.into())
    });

    for (name, script) in scripts.iter().flatten().cloned() {
        ctx.def_fn(name.clone(), move |ctx, _args| {
            println!("{name} start");
            let result = ctx.exec_script(&script);
            println!("{name} => {result:?}");
            result
        });
    }

    let object_ids = content
        .objects
        .items
        .iter()
        .map(|entry| {
            entry.as_ref().map(|o| {
                // not quite right: should be a set of objects based on instances...
                let id = ctx.add_object();
                ctx.set_global(&o.name.0, id);
                id
            })
        })
        .collect::<Vec<_>>();

    for i in &room.instances {
        let o = &content.objects[i.object_index];
        let id = object_ids[i.object_index as usize].unwrap();
        if let Some(create) = o.events.get(&gmk_file::EventId::Create) {
            for action in &create.actions {
                let script = if action.kind == gmk_file::ActionKind::Code {
                    let code = &action.argument_values[0].0;
                    println!("instance {} create inline code", i.id);
                    Some(std::borrow::Cow::Owned(gml::parse(code).unwrap()))
                } else if action.kind == gmk_file::ActionKind::Normal
                    && action.exec == gmk_file::ActionExec::Function
                    && action.function_name.0 == "action_execute_script"
                {
                    let index: usize = action.argument_values[0].0.parse().unwrap();
                    let (name, script) = &scripts[index].as_ref().unwrap();
                    println!("instance {} create script {index} = {name:?}", i.id);
                    Some(std::borrow::Cow::Borrowed(script))
                } else {
                    None
                };
                if let Some(script) = script {
                    match ctx.with_instance(id, |ctx| ctx.exec_script(&script)) {
                        Err(error) => {
                            println!("failed: {error}");
                        }
                        Ok(value) => {
                            println!("result: {value}");
                        }
                    }
                }
            }
        }
    }

    macroquad::Window::from_config(conf(), run_main(ctx, content))
}

async fn run_main(mut ctx: gml::eval::Context, content: gmk_file::Content) {
    let mut loader = Loader::new(&content);

    let room = Room::load(&mut loader, &mut ctx, "rom_main");

    loop {
        room.draw();
        next_frame().await;
    }
}

fn discover_fns(content: &gmk_file::Content) {
    #[derive(Debug, Default)]
    struct Visitor {
        fn_defs: BTreeSet<String>,
        fn_refs: BTreeSet<String>,
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
