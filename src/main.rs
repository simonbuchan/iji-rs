#![deny(rust_2018_idioms)]

use std::collections::BTreeSet;

pub mod gml;

fn main() {
    let content = gmk_file::parse();

    #[derive(Debug, Default)]
    struct Visitor {
        fn_defs: BTreeSet<String>,
        fn_refs: BTreeSet<String>,
    }

    let mut visitor = Visitor::default();

    for (id, source) in enum_scripts(&content) {
        if let ScriptId::Resource(name) = id {
            visitor.fn_defs.insert(name.to_string());
        }
        let file = gml::parse(source).unwrap();
        file.visit(&mut visitor);
    }

    for undef in visitor.fn_refs.difference(&visitor.fn_defs) {
        println!("- {undef}");
    }

    impl gml::Visitor for Visitor {
        fn expr(&mut self, value: &gml::Expr) -> bool {
            if let gml::Expr::Call { id, .. } = value {
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
