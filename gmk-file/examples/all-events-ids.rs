use std::collections::BTreeSet;

fn main() {
    let content = gmk_file::parse("ref/source code/iji.gmk");
    let mut ids = BTreeSet::<gmk_file::EventId>::new();
    for (_, res) in &content.objects {
        ids.extend(res.events.keys());
    }
    for id in ids {
        match id {
            gmk_file::EventId::Collision(object_index) => {
                let (name, _) = &content.objects.item(object_index);
                println!("Collision({name})");
            }
            gmk_file::EventId::KeyPress(code) => {
                if let Ok(id) = gmk_file::KeyboardEventId::try_from(code) {
                    println!("KeyPress({id:?})");
                } else {
                    println!("KeyPress({:?})", char::try_from(code).unwrap());
                }
            }
            gmk_file::EventId::KeyRelease(code) => {
                if let Ok(id) = gmk_file::KeyboardEventId::try_from(code) {
                    println!("KeyRelease({id:?})");
                } else {
                    println!("KeyRelease({:?})", char::try_from(code).unwrap());
                }
            }
            gmk_file::EventId::Keyboard(code) => {
                if let Ok(id) = gmk_file::KeyboardEventId::try_from(code) {
                    println!("Keyboard({id:?})");
                } else {
                    println!("Keyboard({:?})", char::try_from(code).unwrap());
                }
            }
            _ => {
                println!("{id:?}");
            }
        }
    }
}
