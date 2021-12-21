// Parse Verblijfsobject zip file
use serde::de::Deserialize;
use serde_derive::Deserialize;

use std::io::BufReader;
use std::path::Path;

use crate::parse_wrapper::Wrapper;

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
            process_xml(&mut result, reader)?;
        }
    }

    Ok(result)
}

fn process_xml<R: std::io::Read>(result: &mut Verblijfsobjecten, reader: R) -> std::io::Result<()> {
    let reader = BufReader::new(reader);
    let all: Wrapper<BagObject> = quick_xml::de::from_reader(reader).unwrap();

    // println!("size: {}", all.antwoord.producten.product.objects.len());
    let objects = all.objects;

    for object in objects {
        if let BagObject::Verblijfsobject {
            verblijfsobject_geometrie,
            gerelateerde_adressen,
        } = object
        {
            let point = if let Some(point) = verblijfsobject_geometrie.point {
                point.pos
            } else if let Some(polygon) = verblijfsobject_geometrie.polygon {
                let (x, y) = polygon.exterior.linear_ring.pos_list.centroid;
                Geopunt { x, y }
            } else {
                panic!("geometry is not a point nor a polygon")
            };

            result.push(gerelateerde_adressen.hoofdadres.identificatie, point)
        }
    }

    Ok(())
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
pub struct Geopunt {
    pub x: f32,
    pub y: f32,
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
    #[serde(rename = "posList")]
    pos_list: PosList,
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
    Nummeraanduiding {},
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

mod test {
    use super::*;

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
}
