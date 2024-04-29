fn main() {
    let content = gmk_file::parse("ref/source code/iji.gmk");
    for (_, name, res) in &content.sprites {
        for (i, image) in res.subimages.iter().enumerate() {
            let Some(result) = image.parse() else { continue };
            match result {
                Ok((
                    _,
                    gmk_file::ImageData {
                        width,
                        height,
                        bitcount,
                        image_type,
                        data,
                        ..
                    },
                )) => {
                    println!(
                        "{name}[{i}] = {width}x{height}x{bitcount} ({image_type}) {:?}",
                        data.chunks(12).next().unwrap_or_default(),
                    );
                }
                Err(error) => {
                    println!("{name}[{i}] => {error:?}");
                }
            }
        }
    }
}
