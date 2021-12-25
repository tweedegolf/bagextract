// Parse Verblijfsobject zip file
use serde::de::Deserialize;

use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Default)]
pub struct Verblijfsobjecten {
    pub geopunten: Vec<Geopunt>,
    /// postcode id for each geopunt
    pub postcode_id: Vec<u64>,
}

impl Verblijfsobjecten {
    fn push(&mut self, identificatie: u64, geopunt: Geopunt) {
        self.postcode_id.push(identificatie);
        self.geopunten.push(geopunt);
    }
}

pub fn parse(path: &Path) -> std::io::Result<Verblijfsobjecten> {
    let mut result = Verblijfsobjecten::default();

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader).unwrap();

    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();
        let outpath = {
            let f = file.enclosed_name().map(|x| x.to_path_buf());
            match f {
                Some(path) => path.clone(),
                None => {
                    println!("Entry {} has a suspicious path", file.name());
                    continue;
                }
            }
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("Entry {} comment: {}", i, comment);
            }
        }

        if (file.name()).ends_with('/') {
            println!(
                "Entry {} is a directory with name \"{}\"",
                i,
                outpath.display()
            );
        } else {
            println!(
                "Entry {} is a file with name \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );

            let reader = BufReader::new(file);
            // process_xml(&mut result, reader)?;
            parse_manual_step(reader, &mut result).unwrap();
        }
    }

    Ok(result)
}

pub fn parse_manual_str(input: &str) -> Option<Verblijfsobjecten> {
    let mut result = Verblijfsobjecten {
        geopunten: Vec::with_capacity(1024),
        postcode_id: Vec::with_capacity(1024),
    };

    parse_manual_step(input.as_bytes(), &mut result)?;

    Some(result)
}

fn parse_manual_step<B: std::io::BufRead>(input: B, result: &mut Verblijfsobjecten) -> Option<()> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(input);
    let mut buf = Vec::with_capacity(1024);

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if let b"bag_LVC:Verblijfsobject" = e.name() {
                    let object = parse_manual_help(&mut reader, &mut buf)?;
                    result.push(object.identificatie, object.geopunt);
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

#[derive(Debug)]
struct Verblijfsobject2 {
    identificatie: u64,
    geopunt: Geopunt,
}

fn parse_manual_help<B: std::io::BufRead>(
    reader: &mut quick_xml::Reader<B>,
    buf: &mut Vec<u8>,
) -> Option<Verblijfsobject2> {
    use quick_xml::events::Event;
    use std::str::FromStr;

    enum State {
        None,
        Hoofdadres,
        Identificatie,
        Point,
        Polygon,
    }

    let mut state = State::None;

    let mut identificatie = None;
    let mut geopunt = None;

    loop {
        match reader.read_event(buf) {
            Ok(Event::Start(ref e)) => match e.name() {
                b"bag_LVC:hoofdadres" => state = State::Hoofdadres,
                b"bag_LVC:identificatie" => {
                    if let State::Hoofdadres = state {
                        state = State::Identificatie
                    }
                }
                b"gml:pos" => state = State::Point,
                b"gml:posList" => state = State::Polygon,
                _ => (),
            },
            Ok(Event::End(ref e)) => {
                if let b"bag_LVC:Verblijfsobject" = e.name() {
                    match (identificatie, geopunt) {
                        (Some(identificatie), Some(geopunt)) => {
                            return Some(Verblijfsobject2 {
                                identificatie,
                                geopunt,
                            })
                        }
                        _ => return None,
                    }
                }
            }
            Ok(Event::Text(e)) => match state {
                State::None => (),
                State::Hoofdadres => (),
                State::Identificatie => {
                    let string = unsafe { std::str::from_utf8_unchecked(&e) };
                    identificatie = Some(string.parse().unwrap());
                    state = State::None;
                }
                State::Point => {
                    let string = unsafe { std::str::from_utf8_unchecked(&e) };
                    geopunt = Some(Geopunt::from_str(string).unwrap());
                    state = State::None;
                }
                State::Polygon => {
                    let string = unsafe { std::str::from_utf8_unchecked(&e) };
                    let centroid = PosList::from_str(string).unwrap().centroid;
                    geopunt = Some(Geopunt {
                        x: centroid.0,
                        y: centroid.1,
                    });
                    state = State::None;
                }
            },
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => return None,
            _ => (),
        }

        buf.clear();

        if let (Some(identificatie), Some(geopunt)) = (identificatie, geopunt) {
            return Some(Verblijfsobject2 {
                identificatie,
                geopunt,
            });
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Geopunt {
    pub x: f32,
    pub y: f32,
}

impl std::str::FromStr for Geopunt {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.split(' ');

        let x_string = it.next().unwrap();
        let y_string = it.next().unwrap();

        let x: f32 = x_string.parse().unwrap();
        let y: f32 = y_string.parse().unwrap();

        Ok(Geopunt { x, y })
    }
}

/// Custom serde deserializer so we don't create an intermediate string
impl<'de> serde::de::Deserialize<'de> for Geopunt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt;

        struct FieldVisitor;

        impl<'de> serde::de::Visitor<'de> for FieldVisitor {
            type Value = Geopunt;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a postcode")
            }

            fn visit_str<E>(self, value: &str) -> Result<Geopunt, E>
            where
                E: serde::de::Error,
            {
                use std::str::FromStr;

                Ok(Geopunt::from_str(value).unwrap())
            }
        }

        deserializer.deserialize_str(FieldVisitor)
    }
}

#[derive(Debug)]
struct PosList {
    centroid: (f32, f32),
}

impl std::str::FromStr for PosList {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut values = s.split_ascii_whitespace().map(|x| {
            let y: f32 = x.parse().unwrap();
            y
        });

        let mut points = Vec::new();

        while let Some(x) = values.next() {
            let y = values.next().unwrap();
            let _ = values.next().unwrap();

            points.push((x, y));
        }

        let line_string = geo::LineString::from(points);

        let polygon = geo::Polygon::new(line_string, vec![]);

        use geo::algorithm::centroid::Centroid;
        let centroid = polygon.centroid().unwrap();

        let point = (centroid.x(), centroid.y());
        Ok(PosList { centroid: point })
    }
}

impl<'de> Deserialize<'de> for PosList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt;

        struct FieldVisitor;

        impl<'de> serde::de::Visitor<'de> for FieldVisitor {
            type Value = PosList;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a postcode")
            }

            fn visit_str<E>(self, value: &str) -> Result<PosList, E>
            where
                E: serde::de::Error,
            {
                use std::str::FromStr;

                Ok(PosList::from_str(value).unwrap())
            }
        }

        deserializer.deserialize_str(FieldVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_polygon() {
        let input = r#"
                    233392.425  581908.265   0.0
                    233385.577  581905.776   0.0 
                    233390.499  581895.538   0.0 
                    233389.485  581895.102   0.0 
                    233391.734  581889.74    0.0 
                    233392.519  581888.052   0.0 
                    233400.163  581891.489   0.0 
                    233400.211  581891.511   0.0 
                    233399.906  581892.19    0.0 
                    233399.85   581892.168   0.0 
                    233396.991  581898.366   0.0 
                    233392.425  581908.265   0.0
        "#;

        let object: PosList = std::str::FromStr::from_str(input).unwrap();

        dbg!(&object);
    }

    #[test]
    fn geopunt() {
        let input = r#"5.0 3.0 0.0"#;

        let object: Geopunt = std::str::FromStr::from_str(input).unwrap();

        dbg!(&object);
    }

    #[test]
    fn parse_object_manual() {
        let input = r#"
<bag_LVC:Verblijfsobject><bag_LVC:gerelateerdeAdressen><bag_LVC:hoofdadres><bag_LVC:identificatie>0003200000134068</bag_LVC:identificatie></bag_LVC:hoofdadres></bag_LVC:gerelateerdeAdressen><bag_LVC:identificatie>0003010000125996</bag_LVC:identificatie><bag_LVC:aanduidingRecordInactief>N</bag_LVC:aanduidingRecordInactief><bag_LVC:aanduidingRecordCorrectie>0</bag_LVC:aanduidingRecordCorrectie><bag_LVC:officieel>N</bag_LVC:officieel><bag_LVC:verblijfsobjectGeometrie><gml:Point srsName="urn:ogc:def:crs:EPSG::28992">
  <gml:pos>252908.632 593657.117 0.0</gml:pos>
  </gml:Point></bag_LVC:verblijfsobjectGeometrie><bag_LVC:gebruiksdoelVerblijfsobject>kantoorfunctie</bag_LVC:gebruiksdoelVerblijfsobject><bag_LVC:oppervlakteVerblijfsobject>162</bag_LVC:oppervlakteVerblijfsobject><bag_LVC:verblijfsobjectStatus>Verblijfsobject in gebruik</bag_LVC:verblijfsobjectStatus><bag_LVC:tijdvakgeldigheid><bagtype:begindatumTijdvakGeldigheid>2013031300000000</bagtype:begindatumTijdvakGeldigheid><bagtype:einddatumTijdvakGeldigheid>2016050300000000</bagtype:einddatumTijdvakGeldigheid></bag_LVC:tijdvakgeldigheid><bag_LVC:inOnderzoek>N</bag_LVC:inOnderzoek><bag_LVC:bron><bagtype:documentdatum>20130313</bagtype:documentdatum><bagtype:documentnummer>A2013-WFS-015B</bagtype:documentnummer></bag_LVC:bron><bag_LVC:gerelateerdPand><bag_LVC:identificatie>0003100000117644</bag_LVC:identificatie></bag_LVC:gerelateerdPand></bag_LVC:Verblijfsobject>
"#;

        let mut reader = quick_xml::Reader::from_str(input);
        let object: Verblijfsobject2 = parse_manual_help(&mut reader, &mut Vec::new()).unwrap();

        dbg!(&object);
    }
}
