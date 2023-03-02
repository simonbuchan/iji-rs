fn main() {
    let input = std::fs::read_to_string("ref/out/objects/obj_tor/Step-Normal-0.gml").unwrap();
    println!("{:#?}", gml::parse("input", &input));
}
