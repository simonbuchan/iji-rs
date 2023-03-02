use std::collections::BTreeSet;

use gml::ast;

fn main() {
    let content = gmk_file::parse("ref/source code/iji.gmk");

    #[derive(Debug, Default)]
    struct Names {
        defs: BTreeSet<String>,
        refs: BTreeSet<String>,
    }

    impl Names {
        fn iter_undef(&self) -> impl Iterator<Item = &str> {
            self.refs.difference(&self.defs).map(|i| i.as_str())
        }
    }

    #[derive(Debug, Default)]
    struct Visitor {
        fns: Names,
        globals: Names,
        locals: Names,
    }

    impl Visitor {
        fn def_resources<T>(&mut self, chunk: &gmk_file::ResourceChunk<T>) {
            for (name, _) in chunk {
                self.globals.defs.insert(name.to_string());
            }
        }
    }

    let mut visitor = Visitor::default();

    visitor.def_resources(&content.sounds);
    visitor.def_resources(&content.sprites);
    visitor.def_resources(&content.backgrounds);
    visitor.def_resources(&content.paths);
    visitor.def_resources(&content.scripts); // scripts are fns, but can also be used "script_execute(scr_foo)"
    visitor.def_resources(&content.fonts);
    visitor.def_resources(&content.timelines);
    visitor.def_resources(&content.objects);
    visitor.def_resources(&content.rooms);

    for (id, source) in enum_scripts(&content) {
        if let ScriptId::Resource(name) = id {
            visitor.fns.defs.insert(name.into());
        }
        match gml::parse(&format!("{id:?}"), source) {
            Ok(file) => {
                file.visit(&mut visitor);
            }
            Err(error) => {
                eprintln!("failed to parse: {id:?}: {error}");
            }
        }
    }

    // all global defs are also local defs
    visitor
        .locals
        .defs
        .extend(visitor.globals.defs.iter().cloned());

    println!("fns");
    for undef in visitor.fns.iter_undef() {
        println!("- {undef}");
    }

    println!("globals");
    for undef in visitor.globals.iter_undef() {
        println!("- {undef}");
    }

    println!("locals");
    for undef in visitor.locals.iter_undef() {
        println!("- {undef}");
    }

    impl ast::Visitor for Visitor {
        fn assign(&mut self, value: &ast::Assign) -> bool {
            let mut assign_lhs = &*value.lhs;
            let mut def = true;
            let var = loop {
                match assign_lhs {
                    ast::Expr::Var(var) => break var,
                    ast::Expr::Member { lhs, name } => {
                        assign_lhs = lhs;
                        if def {
                            self.locals.defs.insert(name.clone());
                        }
                        def = false;
                    }
                    ast::Expr::Index { lhs, .. } => {
                        assign_lhs = lhs;
                    }
                    ast::Expr::Call { .. } => return true,
                    _ => unreachable!("invalid lhs: {assign_lhs}"),
                }
            };

            if def {
                match var {
                    ast::Var::Global(name) => {
                        self.globals.defs.insert(name.clone());
                    }
                    ast::Var::Local(name) => {
                        self.locals.defs.insert(name.clone());
                    }
                }
            }

            true
        }

        fn expr(&mut self, value: &ast::Expr) -> bool {
            if let ast::Expr::Call { name, .. } = value {
                self.fns.refs.insert(name.clone());
            }
            true
        }

        fn var(&mut self, value: &ast::Var) {
            match value {
                ast::Var::Global(name) => {
                    self.globals.refs.insert(name.clone());
                }
                ast::Var::Local(name) => {
                    self.locals.refs.insert(name.clone());
                }
            }
        }
    }
}

fn enum_scripts(content: &gmk_file::Content) -> impl Iterator<Item = (ScriptId<'_>, &str)> {
    content
        .scripts
        .iter()
        .map(|(name, res)| (ScriptId::Resource(name), res.script.0.as_str()))
        .chain(content.rooms.iter().flat_map(|(name, res)| {
            Some((ScriptId::RoomInit(name), res.creation_code.0.as_str()))
                .into_iter()
                .chain(res.instances.iter().map(|res| {
                    (
                        ScriptId::InstanceInit(name, res.id),
                        res.creation_code.0.as_str(),
                    )
                }))
        }))
        .chain(content.objects.iter().flat_map(|(name, res)| {
            res.events.iter().flat_map(|(id, e)| {
                e.actions.iter().enumerate().filter_map(|(i, a)| {
                    action_code(a).map(|code| (ScriptId::ObjectEvent(name, *id, i), code))
                })
            })
        }))
        .chain(content.timelines.iter().flat_map(|(name, res)| {
            res.moments.iter().flat_map(|m| {
                m.actions.iter().enumerate().filter_map(|(i, a)| {
                    action_code(a).map(|code| (ScriptId::TimelineAction(name, m.position, i), code))
                })
            })
        }))
}

fn action_code(action: &gmk_file::Action) -> Option<&str> {
    match action.kind {
        gmk_file::ActionKind::Code => Some(action.argument_values[0].0.as_str()),
        _ => None,
    }
}

#[derive(Debug)]
enum ScriptId<'a> {
    Resource(&'a str),
    RoomInit(&'a str),
    InstanceInit(&'a str, u32),
    ObjectEvent(&'a str, gmk_file::EventId, usize),
    TimelineAction(&'a str, u32, usize),
}
