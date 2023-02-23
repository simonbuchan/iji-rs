#![deny(rust_2018_idioms)]

use macroquad::prelude::*;

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

// fn write_debug_logs(dir: &str, chunk: &gmk_file::ResourceChunk<impl std::fmt::Debug>) {
//     for (name, item) in chunk {
//         let parent = std::path::Path::new("ref/out").join(dir).join(name);
//         std::fs::create_dir_all(&parent).unwrap();
//         std::fs::write(parent.join("debug.log"), format!("{item:#?}")).unwrap();
//     }
// }

fn main() {
    let content = gmk_file::parse("ref/source code/iji.gmk");

    macroquad::Window::from_config(conf(), run_main(content))
}

async fn run_main(content: gmk_file::Content) {
    let mut ctx = state::Context::new(&content, scripts::create_context());
    ctx.scripts = scripts::define_scripts(&mut ctx.gml, &content);
    for (index, item) in content.objects.items.iter().enumerate() {
        let Some(item) = item else { continue; };

        struct ObjectRef(u32);

        impl gml::eval::Object for ObjectRef {}

        let id = ctx
            .gml
            .new_instance(Some(Box::new(ObjectRef(index as u32))));

        ctx.gml.set_global(item.name.0.clone(), id);
    }
    let mut room = state::Room::load(&mut ctx, "rom_main");

    room.dispatch(&mut ctx, &gmk_file::EventId::Create);

    loop {
        room.dispatch(
            &mut ctx,
            &gmk_file::EventId::Draw(gmk_file::DrawEventId::Normal),
        );
        room.draw(&ctx);

        next_frame().await;
    }
}

mod scripts {
    use std::collections::HashMap;
    use std::sync::Arc;

    use macroquad::prelude::*;
    use rayon::prelude::*;

    pub fn create_context() -> gml::eval::Context {
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
        ctx.def_fn("string_length", |_ctx, args| {
            Ok(i32::try_from(args[0].to_str().len())
                .expect("string too long")
                .into())
        });
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
            let id = ctx.new_instance(None);
            // todo: also add to room instances
            Ok(id.into())
        });

        ctx.def_fn("draw_set_font", |_ctx, _args| Ok(().into()));
        ctx.def_fn("draw_set_color", |_ctx, _args| Ok(().into()));
        ctx.def_fn("draw_text_ext", |_ctx, _args| Ok(().into()));
        ctx.def_fn("draw_sprite", |_ctx, _args| Ok(().into()));

        ctx
    }

    pub fn define_scripts(
        ctx: &mut gml::eval::Context,
        content: &gmk_file::Content,
    ) -> HashMap<u32, Arc<gml::ast::Script>> {
        // scripts parsed in parallel
        let scripts = content
            .scripts
            .items
            .par_iter()
            .enumerate()
            .flat_map(|(index, item)| {
                item.as_ref().map(|item| {
                    let script = gml::parse(&item.name.0, &item.data.script.0).unwrap();
                    (index as u32, item.name.0.clone(), Arc::new(script))
                })
            })
            .collect::<Vec<_>>();

        for (_, name, script) in &scripts {
            let name = name.clone();
            let script = script.clone();
            ctx.def_fn(name.clone(), move |ctx, _args| ctx.exec_script(&script));
        }

        scripts
            .into_iter()
            .map(|(index, _, script)| (index, script))
            .collect()
    }
}

// fn discover_fns(content: &gmk_file::Content) {
//     #[derive(Debug, Default)]
//     struct Visitor {
//         fn_defs: BTreeSet<String>,
//         fn_refs: BTreeSet<String>,
//     }
//
//     let mut visitor = Visitor::default();
//
//     for (id, source) in enum_scripts(&content) {
//         if let ScriptId::Resource(name) = id {
//             visitor.fn_defs.insert(name.into());
//         }
//         let file = gml::parse(name, source).unwrap();
//         file.visit(&mut visitor);
//     }
//
//     for undef in visitor.fn_refs.difference(&visitor.fn_defs) {
//         println!("- {undef}");
//     }
//
//     impl gml::ast::Visitor for Visitor {
//         fn expr(&mut self, value: &gml::ast::Expr) -> bool {
//             if let gml::ast::Expr::Call { id, .. } = value {
//                 self.fn_refs.insert(id.clone());
//             }
//             true
//         }
//     }
// }
//
// fn enum_scripts(content: &gmk_file::Content) -> impl Iterator<Item = (ScriptId<'_>, &str)> {
//     content
//         .scripts
//         .iter()
//         .map(|(name, res)| (ScriptId::Resource(name), res.script.0.as_str()))
// }
//
// enum ScriptId<'a> {
//     Resource(&'a str),
//     RoomInit,
//     InstanceInit,
//     TimelineAction,
// }
