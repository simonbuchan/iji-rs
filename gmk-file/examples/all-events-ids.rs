use std::collections::BTreeSet;

fn main() {
    let content = gmk_file::parse("ref/source code/iji.gmk");
    let mut ids = BTreeSet::<gmk_file::EventId>::new();
    for (_, _, res) in &content.objects {
        ids.extend(res.events.keys());
    }
    for id in ids {
        match id {
            gmk_file::EventId::Collision(object_index) => {
                let (name, _) = &content.objects.item(object_index);
                println!("Collision({name})");
            }
            _ => {
                println!("{id:?}");
            }
        }
    }
}
