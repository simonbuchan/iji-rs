use macroquad::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use crate::assets::*;

fn color_u32(value: u32) -> Color {
    let [a, r, g, b] = value.to_be_bytes();
    Color::from_rgba(r, g, b, a)
}

pub struct Context<'content> {
    pub loader: Loader<'content>,
    pub gml: gml::eval::Context,
    // fixme
    pub scripts: HashMap<u32, Arc<gml::ast::Script>>,
}

impl<'content> Context<'content> {
    pub fn new(content: &'content gmk_file::Content, gml: gml::eval::Context) -> Self {
        Self {
            loader: Loader::new(content),
            gml,
            scripts: Default::default(),
        }
    }
}

pub struct Room {
    view: View,
    background_color: Color,
    background_layers: Vec<Layer>,
    tiles: Vec<Tile>,
    instances: Vec<Instance>,
    foreground_layers: Vec<Layer>,
}

impl Room {
    pub fn new() -> Self {
        Self {
            view: View {
                offset: Vec2::default(),
                size: Vec2::new(screen_width(), screen_height()),
            },
            background_color: Color::default(),
            background_layers: vec![],
            tiles: vec![],
            instances: vec![],
            foreground_layers: vec![],
        }
    }

    pub fn load(ctx: &mut Context<'_>, name: &str) -> Self {
        let mut result = Self::new();

        let def = ctx.loader.content().rooms.get(name).unwrap();
        result.background_color = color_u32(def.background_color);

        for b in &def.backgrounds {
            let Ok(index) = b.background_image_index.try_into() else {
                continue;
            };

            if b.foreground_image.into() {
                &mut result.foreground_layers
            } else {
                &mut result.background_layers
            }
            .push(Layer {
                enabled: b.visible.into(),
                pos: IVec2::new(b.pos.0, b.pos.1).as_vec2(),
                asset: ctx.loader.get_background(index),
                tile: false,
                source: None,
            });
        }

        for t in &def.tiles {
            result.tiles.push(Tile {
                depth: t.depth,
                asset: ctx.loader.get_background(t.background_index),
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
            let pos = IVec2::new(i.pos.0, i.pos.1).as_vec2();
            result.create_instance(ctx, pos, i.object_index);
        }

        result
    }

    pub fn create_instance(&mut self, ctx: &mut Context<'_>, pos: Vec2, object_index: u32) {
        let obj = &ctx.loader.content().objects[object_index];

        assert!(obj.mask_sprite_index < 0);
        assert!(obj.parent_object_index < 0);

        self.instances.push(Instance {
            pos,
            frame: 0,
            visible: obj.visible.into(),
            depth: obj.depth,
            gml_id: ctx.gml.new_instance(),
            object_index,
            sprite_asset: obj
                .sprite_index
                .try_into()
                .ok()
                .map(|index| ctx.loader.get_sprite(index)),
        });
    }

    pub fn draw(&self, ctx: &Context<'_>) {
        clear_background(self.background_color);
        for layer in &self.background_layers {
            layer.draw(ctx.loader.assets(), &self.view);
        }
        for tile in &self.tiles {
            tile.draw(ctx.loader.assets(), &self.view);
        }
        for instance in &self.instances {
            instance.draw(ctx.loader.assets(), &self.view);
        }
        for layer in &self.foreground_layers {
            layer.draw(ctx.loader.assets(), &self.view);
        }
    }

    pub fn dispatch(&mut self, ctx: &mut Context<'_>, event_id: &gmk_file::EventId) {
        for i in &self.instances {
            let id = i.gml_id;
            let def = &ctx.loader.content().objects[i.object_index];
            if let Some(event) = def.events.get(event_id) {
                for action in &event.actions {
                    let script = if action.kind == gmk_file::ActionKind::Code {
                        let code = &action.argument_values[0].0;
                        println!("instance {event_id:?} inline code");
                        Some(std::borrow::Cow::Owned(gml::parse(code).unwrap()))
                    } else if action.kind == gmk_file::ActionKind::Normal
                        && action.exec == gmk_file::ActionExec::Function
                        && action.function_name.0 == "action_execute_script"
                    {
                        let index = action.argument_values[0].0.parse().unwrap();
                        let script = ctx.scripts[&index].as_ref();
                        println!("instance {event_id:?} script {index}");
                        Some(std::borrow::Cow::Borrowed(script))
                    } else {
                        None
                    };
                    if let Some(script) = script {
                        match ctx.gml.with_instance(id, |ctx| ctx.exec_script(&script)) {
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
    }
}

trait Draw {
    fn draw(&self, assets: &Assets, view: &View);
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

impl Draw for Layer {
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

impl Draw for Tile {
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
    gml_id: gml::eval::ObjectId,
    pos: Vec2,
    frame: usize,
}

impl Draw for Instance {
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
