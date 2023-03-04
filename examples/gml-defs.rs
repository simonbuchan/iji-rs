use std::collections::{BTreeSet, HashSet};

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

        fn iter_unused(&self) -> impl Iterator<Item = &str> {
            self.defs.difference(&self.refs).map(|i| i.as_str())
        }
    }

    #[derive(Debug, Default)]
    struct Visitor {
        fns: Names,
        const_defs: HashSet<String>,
        globals: Names,
        locals: Names,
    }

    impl Visitor {
        fn def_resources<T>(&mut self, chunk: &gmk_file::ResourceChunk<T>) {
            for (_, name, _) in chunk {
                self.const_defs.insert(name.to_string());
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

    for (_, action) in enum_actions(&content) {
        if let Some(index) = action_execute_script(action) {
            let (name, _) = content.scripts.item(index);
            visitor.fns.refs.insert(name.into());
        }
    }

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

    // all global refs and defs are also local refs and defs
    visitor
        .locals
        .defs
        .extend(visitor.globals.defs.iter().cloned());
    visitor
        .locals
        .refs
        .extend(visitor.globals.refs.iter().cloned());

    let groups = [
        ("fns", &visitor.fns),
        ("globals", &visitor.globals),
        ("locals", &visitor.locals),
    ];
    for (group, names) in groups {
        println!("{group} undef");
        for name in names.iter_undef() {
            if !visitor.const_defs.contains(name) {
                println!("- {name}");
            }
        }
        println!("{group} unused");
        for name in names.iter_unused() {
            println!("- {name}");
        }
    }

    impl Visitor {
        fn visit_call(&mut self, name: &str, args: &[Box<ast::Expr>]) {
            self.fns.refs.insert(name.into());
            if name == "script_execute" {
                if let Some(arg) = args.get(0) {
                    if let ast::Expr::Var(ast::Var::Local(name)) = arg.as_ref() {
                        self.fns.refs.insert(name.clone());
                    }
                }
            }
        }
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
                    ast::Expr::Call { name, args, .. } => {
                        self.visit_call(name, args);
                        return false;
                    }
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

            !def
        }

        fn expr(&mut self, value: &ast::Expr) -> bool {
            if let ast::Expr::Call { name, args, .. } = value {
                self.visit_call(name, args);
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
        .map(|(_, name, res)| (ScriptId::Resource(name), res.script.0.as_str()))
        .chain(content.rooms.iter().flat_map(|(_, name, res)| {
            Some((ScriptId::RoomInit(name), res.creation_code.0.as_str()))
                .into_iter()
                .chain(res.instances.iter().map(|res| {
                    (
                        ScriptId::InstanceInit(name, res.id),
                        res.creation_code.0.as_str(),
                    )
                }))
        }))
        .chain(
            enum_actions(content)
                .filter_map(|(id, a)| action_code(a).map(|code| (ScriptId::Action(id), code))),
        )
}

fn enum_actions(
    content: &gmk_file::Content,
) -> impl Iterator<Item = (ActionId<'_>, &gmk_file::Action)> {
    content
        .objects
        .iter()
        .flat_map(|(_, name, res)| {
            res.events.iter().flat_map(|(id, e)| {
                e.actions
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (ActionId::ObjectEvent(name, *id, i), a))
            })
        })
        .chain(content.timelines.iter().flat_map(|(_, name, res)| {
            res.moments.iter().flat_map(|m| {
                m.actions
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (ActionId::Timeline(name, m.position, i), a))
            })
        }))
}

fn action_code(action: &gmk_file::Action) -> Option<&str> {
    if action.kind == gmk_file::ActionKind::Code {
        Some(action.argument_values[0].0.as_str())
    } else {
        None
    }
}

fn action_execute_script(action: &gmk_file::Action) -> Option<u32> {
    if action.kind == gmk_file::ActionKind::Normal
        && action.exec == gmk_file::ActionExec::Function
        && action.function_name.0.as_str() == "action_execute_script"
    {
        action.argument_values.get(0)?.parse().ok()
    } else {
        None
    }
}

#[derive(Debug)]
enum ScriptId<'a> {
    Resource(&'a str),
    RoomInit(&'a str),
    InstanceInit(&'a str, u32),
    Action(ActionId<'a>),
}

#[derive(Debug)]
enum ActionId<'a> {
    ObjectEvent(&'a str, gmk_file::EventId, usize),
    Timeline(&'a str, u32, usize),
}
