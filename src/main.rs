use chrono::{Date, DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::de::Deserialize;
use serde_derive::Deserialize;
use std::io::BufReader;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn main() -> std::io::Result<()> {
    // let f = std::fs::File::open("/home/folkertdev/tg/pect/bagextract/single.xml").unwrap();
    // let mut reader = BufReader::new(f);
    // let all: Wrapper = quick_xml::de::from_reader(reader).unwrap();

    extract()
}

fn extract() -> std::io::Result<()> {
    // process_file(path)
    if let Ok(it) = std::fs::read_dir("/home/folkertdev/Downloads/inspire/") {
        for x in it {
            let path = x.unwrap().path();
            let state = process_file(&path)?;

            dbg!(
                path,
                state.adressen.geopunten.len(),
                state.postcodes.postcodes.len()
            );
        }
    }
    Ok(())
}

fn process_file(file_path: &Path) -> std::io::Result<State> {
    let file = std::fs::File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut state = State::default();

    process_file_help(&mut state, file_path, reader)?;

    Ok(state)
}

fn process_file_help<R: Read + Seek>(
    state: &mut State,
    file_path: &Path,
    reader: R,
) -> std::io::Result<()> {
    match file_path.extension().and_then(|x| x.to_str()) {
        Some("zip") => process_zip(state, reader),
        Some("xml") => {
            println!("ignoring {:?}", file_path);
            Ok(())
        }
        Some("csv") => {
            println!("ignoring {:?}", file_path);
            Ok(())
        }
        Some(_) | None => {
            println!("ignoring {:?}", file_path);
            Ok(())
        }
    }
}

fn process_file_help2<R: Read>(
    state: &mut State,
    file_path: &Path,
    reader: R,
) -> std::io::Result<()> {
    match file_path.extension().and_then(|x| x.to_str()) {
        Some("zip") => panic!("nested too deep"),
        Some("xml") => {
            if file_path.starts_with("GEM-WPL-RELATIE") {
                Ok(())
            } else {
                process_xml(state, reader)
            }
        }
        Some("csv") => {
            println!("ignoring {:?}", file_path);
            Ok(())
        }
        Some(_) | None => {
            println!("ignoring {:?}", file_path);
            Ok(())
        }
    }
}

fn process_xml<R: Read>(state: &mut State, reader: R) -> std::io::Result<()> {
    let reader = BufReader::new(reader);
    let all: Wrapper = quick_xml::de::from_reader(reader).unwrap();

    // println!("size: {}", all.antwoord.producten.product.objects.len());
    let objects = all.antwoord.producten.product.objects;

    for object in objects {
        println!(
            "lengths: {} {}",
            state.adressen.geopunten.len(),
            state.postcodes.postcodes.len()
        );
        match object {
            BagObject::Verblijfsobject {
                verblijfsobject_geometrie,
                gerelateerde_adressen,
            } => {
                let point = if let Some(point) = verblijfsobject_geometrie.point {
                    point.pos
                } else if let Some(polygon) = verblijfsobject_geometrie.polygon {
                    let (x, y) = polygon.exterior.linear_ring.posList.centroid;
                    Geopunt { x, y }
                } else {
                    panic!("geometry is not a point nor a polygon")
                };

                state
                    .adressen
                    .push(gerelateerde_adressen.hoofdadres.identificatie, point)
            }
            BagObject::Nummeraanduiding {
                postcode,
                identificatie,
            } => match postcode {
                None => {
                    println!(
                        "skipping nummeraanduiding {}, it has no postcode",
                        identificatie
                    );
                }
                Some(postcode) => {
                    let postcode = CompactPostcode::try_from(postcode.as_str()).unwrap();
                    state.postcodes.push(identificatie, postcode);
                }
            },
            _ => {}
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
struct Adressen {
    geopunten: Vec<Geopunt>,
    /// postcode id for each geopunt
    links: Vec<u64>,
}

#[derive(Debug, Default)]
struct State {
    postcodes: Postcodes,
    adressen: Adressen,
}

#[derive(Debug, Default)]
struct Postcodes {
    identificatie: Vec<u64>,
    postcodes: Vec<CompactPostcode>,
}

impl Postcodes {
    fn push(&mut self, identificatie: u64, postcode: CompactPostcode) {
        self.identificatie.push(identificatie);
        self.postcodes.push(postcode);
    }
}

impl Adressen {
    fn push(&mut self, identificatie: u64, geopunt: Geopunt) {
        self.links.push(identificatie);
        self.geopunten.push(geopunt);
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct CompactPostcode {
    digits: u16,
    letters: [u8; 2],
}

impl std::fmt::Debug for CompactPostcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompactPostcode")
            .field("digits", &self.digits)
            .field("letters", &self.letters)
            .field(
                "pretty",
                &format!(
                    "{} {}{}",
                    self.digits, self.letters[0] as char, self.letters[1] as char
                ),
            )
            .finish()
    }
}

impl TryFrom<&str> for CompactPostcode {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 6 {
            return Err(());
        }

        let digits: u16 = match &value[..4].parse() {
            Ok(v) => *v,
            Err(e) => {
                panic!("{}", e);
            }
        };

        let letters: [u8; 2] = value[4..6].as_bytes().try_into().unwrap();

        Ok(CompactPostcode { digits, letters })
    }
}

fn process_zip<R: Read + Seek>(state: &mut State, reader: R) -> std::io::Result<()> {
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

            // let mut string = String::new();
            // let mut file = file;
            // file.read_to_string(&mut string);
            // println!("{}", string);

            process_file_help2(state, &outpath, file)?;
        }
    }

    Ok(())
}

fn prefix_bag_lvc(name: &str) -> String {
    format!("bag_LVC:{}", name)
}

#[derive(Debug, Deserialize)]
#[serde(rename = "xb:BAG-Extract-Deelbestand-LVC")]
struct Wrapper {
    antwoord: Antwoord,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "xb:antwoord")]
struct Antwoord {
    producten: Producten,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "xb:producten")]
struct Producten {
    #[serde(alias = "LVC-product")]
    product: Product,
}

#[derive(Debug, Deserialize)]
struct Product {
    #[serde(rename = "$value")]
    objects: Vec<BagObject>,
}

#[derive(Debug, Deserialize)]
enum BagObject {
    #[serde(rename = "bag_LVC:VerblijfsObjectPand")]
    VerblijfsObjectPand {},
    #[serde(rename = "bag_LVC:AdresseerbaarObjectNevenAdres")]
    AdresseerbaarObjectNevenAdres {},
    #[serde(rename = "bag_LVC:VerblijfsObjectGebruiksdoel")]
    VerblijfsObjectGebruiksdoel {},
    #[serde(rename = "bag_LVC:Woonplaats")]
    Woonplaats {},
    #[serde(rename = "bag_LVC:OpenbareRuimte")]
    OpenbareRuimte {},
    #[serde(rename = "bag_LVC:Nummeraanduiding")]
    Nummeraanduiding {
        identificatie: u64,
        postcode: Option<String>,
    },
    #[serde(rename = "bag_LVC:Ligplaats")]
    Ligplaats {},
    #[serde(rename = "bag_LVC:Standplaats")]
    Standplaats {},
    #[serde(rename = "bag_LVC:Verblijfsobject")]
    #[serde(rename_all = "camelCase")]
    Verblijfsobject {
        gerelateerde_adressen: GerelateerdeAdressen,
        verblijfsobject_geometrie: VerblijfsobjectGeometrie,
    },
    #[serde(rename = "bag_LVC:Pand")]
    Pand {},
}

#[derive(Debug, Deserialize)]
struct GerelateerdeAdressen {
    hoofdadres: Hoofdadres,
}

#[derive(Debug, Deserialize)]
struct Hoofdadres {
    identificatie: u64,
}
#[derive(Debug, Deserialize)]
struct VerblijfsobjectGeometrie {
    #[serde(alias = "Point")]
    point: Option<Point>,
    #[serde(alias = "Polygon")]
    polygon: Option<Polygon>,
}

#[derive(Debug, Deserialize)]
struct Point {
    pos: Geopunt,
}

#[derive(Debug)]
struct Geopunt {
    x: f32,
    y: f32,
}

impl<'de> Deserialize<'de> for Geopunt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;

        let mut it = string.split(' ');

        let x_string = it.next().unwrap();
        let y_string = it.next().unwrap();

        let x: f32 = x_string.parse().unwrap();
        let y: f32 = y_string.parse().unwrap();

        Ok(Geopunt { x, y })
    }
}

#[derive(Debug, Deserialize)]
struct Polygon {
    exterior: Exterior,
}

#[derive(Debug, Deserialize)]
struct Exterior {
    #[serde(rename = "LinearRing")]
    linear_ring: LinearRing,
}

#[derive(Debug, Deserialize)]
struct LinearRing {
    posList: PosList,
}

#[derive(Debug)]
struct PosList {
    centroid: (f32, f32),
}

impl<'de> Deserialize<'de> for PosList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;

        let mut values = string.split_ascii_whitespace().map(|x| {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_verblijfsobject() {
        let input = r#"
<bag_LVC:Verblijfsobject><bag_LVC:gerelateerdeAdressen><bag_LVC:hoofdadres><bag_LVC:identificatie>0000200000057534</bag_LVC:identificatie></bag_LVC:hoofdadres></bag_LVC:gerelateerdeAdressen>

<bag_LVC:identificatie>0000010000057469</bag_LVC:identificatie><bag_LVC:aanduidingRecordInactief>N</bag_LVC:aanduidingRecordInactief><bag_LVC:aanduidingRecordCorrectie>0</bag_LVC:aanduidingRecordCorrectie><bag_LVC:officieel>N</bag_LVC:officieel>


<bag_LVC:verblijfsobjectGeometrie><gml:Point srsName="urn:ogc:def:crs:EPSG::28992">
  <gml:pos>188391.884 334586.439 0.0</gml:pos>
</gml:Point></bag_LVC:verblijfsobjectGeometrie>



<bag_LVC:gebruiksdoelVerblijfsobject>woonfunctie</bag_LVC:gebruiksdoelVerblijfsobject><bag_LVC:oppervlakteVerblijfsobject>72</bag_LVC:oppervlakteVerblijfsobject><bag_LVC:verblijfsobjectStatus>Verblijfsobject in gebruik</bag_LVC:verblijfsobjectStatus><bag_LVC:tijdvakgeldigheid><bagtype:begindatumTijdvakGeldigheid>2018032600000000</bagtype:begindatumTijdvakGeldigheid><bagtype:einddatumTijdvakGeldigheid>2018040400000000</bagtype:einddatumTijdvakGeldigheid></bag_LVC:tijdvakgeldigheid><bag_LVC:inOnderzoek>N</bag_LVC:inOnderzoek><bag_LVC:bron><bagtype:documentdatum>20180326</bagtype:documentdatum><bagtype:documentnummer>BV05.00043-HLG</bagtype:documentnummer></bag_LVC:bron><bag_LVC:gerelateerdPand><bag_LVC:identificatie>1883100000010452</bag_LVC:identificatie></bag_LVC:gerelateerdPand></bag_LVC:Verblijfsobject>
"#;

        let object: BagObject = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }

    #[test]
    fn wrapper() {
        let input = r#"
                <xb:BAG-Extract-Deelbestand-LVC>
                  <xb:antwoord>
                    <xb:vraag>
                      <selecties-extract:Gebied-Registratief>
                        <selecties-extract:Gebied-NLD>
                          <selecties-extract:GebiedIdentificatie>9999</selecties-extract:GebiedIdentificatie>
                          <selecties-extract:GebiedNaam>Nederland</selecties-extract:GebiedNaam>
                          <selecties-extract:gebiedTypeNederland>1</selecties-extract:gebiedTypeNederland>
                        </selecties-extract:Gebied-NLD>
                      </selecties-extract:Gebied-Registratief>
                      <selecties-extract:StandTechnischeDatum>20211008</selecties-extract:StandTechnischeDatum>
                    </xb:vraag>
                    <xb:producten>
                        <product_LVC:LVC-product>
                            <bag_LVC:Verblijfsobject><bag_LVC:gerelateerdeAdressen><bag_LVC:hoofdadres><bag_LVC:identificatie>0000200000057534</bag_LVC:identificatie></bag_LVC:hoofdadres></bag_LVC:gerelateerdeAdressen><bag_LVC:identificatie>0000010000057469</bag_LVC:identificatie><bag_LVC:aanduidingRecordInactief>N</bag_LVC:aanduidingRecordInactief><bag_LVC:aanduidingRecordCorrectie>0</bag_LVC:aanduidingRecordCorrectie><bag_LVC:officieel>N</bag_LVC:officieel><bag_LVC:verblijfsobjectGeometrie><gml:Point srsName="urn:ogc:def:crs:EPSG::28992">
                              <gml:pos>188391.884 334586.439 0.0</gml:pos>
                            </gml:Point></bag_LVC:verblijfsobjectGeometrie><bag_LVC:gebruiksdoelVerblijfsobject>woonfunctie</bag_LVC:gebruiksdoelVerblijfsobject><bag_LVC:oppervlakteVerblijfsobject>72</bag_LVC:oppervlakteVerblijfsobject><bag_LVC:verblijfsobjectStatus>Verblijfsobject in gebruik</bag_LVC:verblijfsobjectStatus><bag_LVC:tijdvakgeldigheid><bagtype:begindatumTijdvakGeldigheid>2018032600000000</bagtype:begindatumTijdvakGeldigheid><bagtype:einddatumTijdvakGeldigheid>2018040400000000</bagtype:einddatumTijdvakGeldigheid></bag_LVC:tijdvakgeldigheid><bag_LVC:inOnderzoek>N</bag_LVC:inOnderzoek><bag_LVC:bron><bagtype:documentdatum>20180326</bagtype:documentdatum><bagtype:documentnummer>BV05.00043-HLG</bagtype:documentnummer></bag_LVC:bron><bag_LVC:gerelateerdPand><bag_LVC:identificatie>1883100000010452</bag_LVC:identificatie></bag_LVC:gerelateerdPand></bag_LVC:Verblijfsobject>
                        </product_LVC:LVC-product>
                    </xb:producten>
                  </xb:antwoord>
                </xb:BAG-Extract-Deelbestand-LVC>
            "#;

        let object: Wrapper = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }

    #[test]
    fn parse_polygon() {
        let input = r#"
                <gml:Polygon srsName="urn:ogc:def:crs:EPSG::28992"><gml:exterior><gml:LinearRing><gml:posList srsDimension="3" count="12"> 
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
                </gml:posList></gml:LinearRing></gml:exterior></gml:Polygon>
        "#;

        let object: Polygon = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }

    #[test]
    fn parse_nummeraanduiding() {
        let input = r#"
            <bag_LVC:Nummeraanduiding><bag_LVC:identificatie>0000200000057534</bag_LVC:identificatie><bag_LVC:aanduidingRecordInactief>N</bag_LVC:aanduidingRecordInactief><bag_LVC:aanduidingRecordCorrectie>0</bag_LVC:aanduidingRecordCorrectie><bag_LVC:huisnummer>32</bag_LVC:huisnummer><bag_LVC:officieel>N</bag_LVC:officieel><bag_LVC:huisletter>A</bag_LVC:huisletter><bag_LVC:postcode>6131BE</bag_LVC:postcode><bag_LVC:tijdvakgeldigheid><bagtype:begindatumTijdvakGeldigheid>2018032600000000</bagtype:begindatumTijdvakGeldigheid><bagtype:einddatumTijdvakGeldigheid>2018040400000000</bagtype:einddatumTijdvakGeldigheid></bag_LVC:tijdvakgeldigheid><bag_LVC:inOnderzoek>N</bag_LVC:inOnderzoek><bag_LVC:typeAdresseerbaarObject>Verblijfsobject</bag_LVC:typeAdresseerbaarObject><bag_LVC:bron><bagtype:documentdatum>20180326</bagtype:documentdatum><bagtype:documentnummer>BV05.00043-HLG</bagtype:documentnummer></bag_LVC:bron><bag_LVC:nummeraanduidingStatus>Naamgeving uitgegeven</bag_LVC:nummeraanduidingStatus><bag_LVC:gerelateerdeOpenbareRuimte><bag_LVC:identificatie>1883300000001522</bag_LVC:identificatie></bag_LVC:gerelateerdeOpenbareRuimte></bag_LVC:Nummeraanduiding>
"#;

        let object: BagObject = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }
}
