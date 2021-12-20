use std::collections::hash_map::HashMap;
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    // let f = std::fs::File::open("/home/folkertdev/tg/pect/bagextract/single.xml").unwrap();
    // let mut reader = BufReader::new(f);
    // let all: Wrapper = quick_xml::de::from_reader(reader).unwrap();

    extract()
}

mod parse_num;
mod parse_vbo;

use parse_num::CompactPostcode;

fn extract() -> std::io::Result<()> {
    let verblijfsobjecten_path =
        PathBuf::from("/home/folkertdev/Downloads/inspire/9999VBO08102021.zip");
    let nummeraanduidingen_path =
        PathBuf::from("/home/folkertdev/Downloads/inspire/9999NUM08102021.zip");

    match (
        parse_vbo::parse(&verblijfsobjecten_path),
        parse_num::parse(&nummeraanduidingen_path),
    ) {
        (Ok(verblijfsobjecten), Ok(nummeraanduidingen)) => {
            let it = nummeraanduidingen
                .identificatie
                .into_iter()
                .zip(nummeraanduidingen.postcodes.into_iter());
            let map: HashMap<u64, CompactPostcode> = it.collect();

            let it = verblijfsobjecten
                .postcode_id
                .into_iter()
                .zip(verblijfsobjecten.geopunten.into_iter())
                .filter_map(|(id, geopunt)| match map.get(&id) {
                    None => None,
                    Some(postcode) => Some((geopunt, postcode)),
                });

            let result: Vec<_> = it.collect();

            dbg!(&result);
        }
        _ => panic!(),
    }

    Ok(())
}
