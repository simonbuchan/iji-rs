fn main() {
    let content = gmk_file::parse("ref/source code/iji.gmk");
    dump_tree(0, &content.resource_tree);
}

fn dump_tree(indent: usize, items: &[gmk_file::ResourceTreeItem]) {
    for item in items {
        println!("{:indent$}[{}] {}", "", item.index, item.name);
        dump_tree(indent + 2, &item.contents);
    }
}
