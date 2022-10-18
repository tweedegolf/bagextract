// Parse Nummeraanduiding zip file

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::postcode::Postcode;

#[derive(Debug, Default)]
pub struct Postcodes {
    pub identificatie: Vec<u64>,
    pub postcodes: Vec<Postcode>,
}

impl Postcodes {
    fn push(&mut self, identificatie: u64, postcode: Postcode) {
        self.identificatie.push(identificatie);
        self.postcodes.push(postcode);
    }

    fn merge(mut self, other: Self) -> Self {
        self.identificatie.extend(other.identificatie);
        self.postcodes.extend(other.postcodes);

        self
    }
}

pub fn parse(path: &Path) -> std::io::Result<Postcodes> {
    let file = std::fs::File::open(path)?;
    let archive = zip::ZipArchive::new(file).unwrap();

    let range = 0..archive.len();
    // let range = 0..10;

    let result = parse_step(path, range.start, range.end)?;

    Ok(result)
}

fn parse_ith_xml_file(archive: &mut zip::ZipArchive<File>, i: usize) -> Option<Postcodes> {
    let file = archive.by_index(i).unwrap();

    if file.name().ends_with('/') {
        println!("Entry {} is a directory with name \"{}\"", i, file.name());
        None
    } else {
        println!(
            "Entry {} is a file with name \"{}\" ({} bytes)",
            i,
            file.name(),
            file.size()
        );

        let reader = BufReader::new(file);
        let mut result = Postcodes::default();
        parse_manual_step(reader, &mut result).unwrap();

        Some(result)
    }
}

fn parse_step(path: &Path, start: usize, end: usize) -> std::io::Result<Postcodes> {
    use rayon::prelude::*;

    let init = || {
        let file = std::fs::File::open(path).unwrap();
        zip::ZipArchive::new(file).unwrap()
    };

    let result = (start..end)
        .into_par_iter()
        .map_init(init, parse_ith_xml_file)
        .filter_map(|x| x)
        .reduce(Postcodes::default, Postcodes::merge);

    Ok(result)
}

#[derive(Debug)]
pub struct Nummeraanduiding {
    identificatie: u64,
    postcode: Option<Postcode>,
}

pub fn parse_manual_str(input: &str) -> Option<Postcodes> {
    let mut result = Postcodes {
        identificatie: Vec::with_capacity(10_000),
        postcodes: Vec::with_capacity(10_000),
    };

    parse_manual_step(input.as_bytes(), &mut result)?;

    Some(result)
}

fn parse_manual_step<B: std::io::BufRead>(input: B, result: &mut Postcodes) -> Option<()> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(input);
    let mut buf = Vec::with_capacity(1024);

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if let b"Objecten:Nummeraanduiding" = e.name() {
                    let aanduiding = parse_manual_help(&mut reader, &mut buf)?;
                    if let Some(postcode) = aanduiding.postcode {
                        // println!("identificatie {:?}", aanduiding.identificatie);
                        result.push(aanduiding.identificatie, postcode);
                    }
                }
            }
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            _ => (),
        }

        buf.clear();
    }

    Some(())
}

fn parse_manual_help<B: std::io::BufRead>(
    reader: &mut quick_xml::Reader<B>,
    buf: &mut Vec<u8>,
) -> Option<Nummeraanduiding> {
    use quick_xml::events::Event;

    enum State {
        None,
        Identificatie,
        Postcode,
    }

    let mut state = State::None;

    let mut identificatie = None;
    let mut postcode = None;

    loop {
        match reader.read_event(buf) {
            Ok(Event::Start(ref e)) => match e.name() {
                b"Objecten:identificatie" => state = State::Identificatie,
                b"Objecten:postcode" => state = State::Postcode,
                _ => (),
            },
            Ok(Event::End(ref e)) => {
                if let b"Objecten:Nummeraanduiding" = e.name() {
                    match identificatie {
                        Some(identificatie) => {
                            return Some(Nummeraanduiding {
                                identificatie,
                                postcode,
                            })
                        }
                        None => return None,
                    }
                }
            }
            Ok(Event::Text(e)) => match state {
                State::None => (),
                State::Identificatie => {
                    let string = unsafe { std::str::from_utf8_unchecked(&e) };
                    identificatie = Some(string.parse().unwrap());
                    state = State::None;
                }
                State::Postcode => {
                    let string = unsafe { std::str::from_utf8_unchecked(&e) };
                    postcode = Some(Postcode::try_from(string).unwrap());
                    state = State::None;
                }
            },
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => return None,
            _ => (),
        }

        buf.clear();

        if let (Some(identificatie), Some(postcode)) = (identificatie, postcode) {
            return Some(Nummeraanduiding {
                identificatie,
                postcode: Some(postcode),
            });
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /*
        #[test]
        fn parse_verblijfsobject() {
            let input = r#"
    "#;

            let object: Nummeraanduiding = quick_xml::de::from_str(input).unwrap();

            dbg!(&object);
        }

        #[test]
        fn wrapper() {
            let input = r#"
                "#;

            let object: Wrapper<Nummeraanduiding> = quick_xml::de::from_str(input).unwrap();

            dbg!(&object);
        }

        #[test]
        fn parse_nummeraanduiding() {
            let input = r#"

    "#;

            let object: Nummeraanduiding = quick_xml::de::from_str(input).unwrap();

            dbg!(&object);
        }
        */

    #[test]
    fn parse_nummeraanduiding_manual() {
        let input = r#"
"#;

        let mut reader = quick_xml::Reader::from_str(input);
        let object: Nummeraanduiding = parse_manual_help(&mut reader, &mut Vec::new()).unwrap();

        dbg!(&object);
    }

    #[test]
    fn parse_nummeraanduiding_many_manual() {
        const INPUT: &str = include_str!("/home/folkertdev/Downloads/inspire/num_01.xml");

        let object: Postcodes = parse_manual_str(INPUT).unwrap();

        // 10_000 elements are parsed, but some don't have a postcode
        assert_eq!(9376, object.postcodes.len());
    }
}
