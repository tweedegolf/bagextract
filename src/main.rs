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
            process_file(&x.unwrap().path())?;
        }
    }
    Ok(())
}

fn process_file(file_path: &Path) -> std::io::Result<()> {
    let file = std::fs::File::open(file_path)?;
    let reader = BufReader::new(file);

    process_file_help(file_path, reader)
}

fn process_file_help<R: Read + Seek>(file_path: &Path, reader: R) -> std::io::Result<()> {
    match file_path.extension().and_then(|x| x.to_str()) {
        Some("zip") => process_zip(reader),
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

fn process_file_help2<R: Read>(file_path: &Path, reader: R) -> std::io::Result<()> {
    match file_path.extension().and_then(|x| x.to_str()) {
        Some("zip") => panic!("nested too deep"),
        Some("xml") => {
            if file_path.starts_with("GEM-WPL-RELATIE") {
                Ok(())
            } else {
                process_xml(reader)
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

fn process_xml<R: Read>(reader: R) -> std::io::Result<()> {
    let reader = BufReader::new(reader);
    let all: Wrapper = quick_xml::de::from_reader(reader).unwrap();

    // println!("size: {}", all.antwoord.producten.product.objects.len());
    let _ = all.antwoord.producten.product.objects.len();

    Ok(())
}

fn process_zip<R: Read + Seek>(reader: R) -> std::io::Result<()> {
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
            process_file_help2(&outpath, file)?;
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
struct BagObjectCommon {
    #[serde(alias = "identificatie")]
    identificatie: String,
    #[serde(alias = "aanduidingRecordInactief")]
    #[serde(deserialize_with = "sad_boolean")]
    aanduiding_record_inactief: bool,
    #[serde(alias = "aanduidingRecordCorrectie")]
    aanduiding_record_correctie: i64,
    #[serde(alias = "officieel")]
    #[serde(deserialize_with = "sad_boolean")]
    officieel: bool,
    #[serde(alias = "inOnderzoek")]
    #[serde(deserialize_with = "sad_boolean")]
    in_onderzoek: bool,
    tijdvakgeldigheid: TijdvakGeldigheid,
    bron: Bron,
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
        identificatie: String,
        #[serde(deserialize_with = "sad_boolean")]
        aanduiding_record_inactief: bool,
        aanduiding_record_correctie: i64,
        #[serde(deserialize_with = "sad_boolean")]
        officieel: bool,
        #[serde(deserialize_with = "sad_boolean")]
        in_onderzoek: bool,
        tijdvakgeldigheid: TijdvakGeldigheid,
        bron: Bron,
    },
    #[serde(rename = "bag_LVC:Pand")]
    Pand {},
}

#[derive(Debug, Deserialize)]
struct Bron {
    documentnummer: String,
    #[serde(deserialize_with = "decode_date")]
    documentdatum: NaiveDate,
}

#[derive(Debug, Deserialize)]
struct TijdvakGeldigheid {
    #[serde(alias = "begindatumTijdvakGeldigheid")]
    #[serde(deserialize_with = "decode_datetime")]
    begindatum_tijdvak_geldigheid: DateTime<Utc>,
    #[serde(alias = "einddatumTijdvakGeldigheid")]
    #[serde(deserialize_with = "decode_datetime")]
    einddatum_tijdvak_geldigheid: DateTime<Utc>,
}

fn decode_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;
    Ok(parse_date(&string))
}

fn decode_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;
    let naive = parse_datetime(&string);
    Ok(DateTime::from_utc(naive, Utc))
}

fn sad_boolean<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let sad = String::deserialize(deserializer)?;

    if sad == "N" {
        Ok(false)
    } else if sad == "Y" {
        Ok(true)
    } else {
        dbg!(sad);
        todo!()
    }
}

fn parse_date(input: &str) -> NaiveDate {
    debug_assert_eq!(input.len(), "20180326".len());

    let year_str = &input[0..4];
    let month_str = &input[4..6];
    let day_str = &input[6..8];

    let year: i32 = year_str.parse().unwrap();
    let month: u32 = month_str.parse().unwrap();
    let day: u32 = day_str.parse().unwrap();

    NaiveDate::from_ymd(year, month, day)
}

fn parse_time(input: &str) -> NaiveTime {
    debug_assert_eq!(input.len(), "00000000".len());

    let h_str = &input[0..2];
    let m_str = &input[2..4];
    let s_str = &input[4..6];
    let mm_str = &input[6..8];

    let h: u32 = h_str.parse().unwrap();
    let m: u32 = m_str.parse().unwrap();
    let s: u32 = s_str.parse().unwrap();
    let mm: u32 = mm_str.parse().unwrap();

    NaiveTime::from_hms_milli(h, m, s, mm)
}

fn parse_datetime(input: &str) -> NaiveDateTime {
    debug_assert_eq!(input.len(), "2018032600000000".len());

    let date = parse_date(&input[..8]);
    let time = parse_time(&input[8..]);

    NaiveDateTime::new(date, time)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_officieel() {
        let input = r#" 
            <Verblijfsobject>
                <officieel>N</officieel> 
            </Verblijfsobject>
            "#;

        #[derive(Debug, Deserialize)]
        struct SadBoolean {
            #[serde(rename = "Y")]
            yes: Option<()>,
            #[serde(rename = "N")]
            no: Option<()>,
        }

        #[derive(Debug, Deserialize)]
        struct Verblijfsobject {
            officieel: SadBoolean,
        }

        let object: Verblijfsobject = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }

    #[test]
    fn parse_tijdvakgeldigheid() {
        let input = r#"
<bag_LVC:tijdvakgeldigheid><bagtype:begindatumTijdvakGeldigheid>2018032600000000</bagtype:begindatumTijdvakGeldigheid><bagtype:einddatumTijdvakGeldigheid>2018040400000000</bagtype:einddatumTijdvakGeldigheid></bag_LVC:tijdvakgeldigheid>
"#;

        let object: TijdvakGeldigheid = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }

    #[test]
    fn test_parse_datetime() {
        // format: JJJJMMDDUUMMSSmm
        let input = "2018032600000000";
    }

    #[test]
    fn parse_verblijfsobject() {
        let input = r#"
<bag_LVC:Verblijfsobject><bag_LVC:gerelateerdeAdressen><bag_LVC:hoofdadres><bag_LVC:identificatie>0000200000057534</bag_LVC:identificatie></bag_LVC:hoofdadres></bag_LVC:gerelateerdeAdressen><bag_LVC:identificatie>0000010000057469</bag_LVC:identificatie><bag_LVC:aanduidingRecordInactief>N</bag_LVC:aanduidingRecordInactief><bag_LVC:aanduidingRecordCorrectie>0</bag_LVC:aanduidingRecordCorrectie><bag_LVC:officieel>N</bag_LVC:officieel><bag_LVC:verblijfsobjectGeometrie><gml:Point srsName="urn:ogc:def:crs:EPSG::28992">
  <gml:pos>188391.884 334586.439 0.0</gml:pos>
</gml:Point></bag_LVC:verblijfsobjectGeometrie><bag_LVC:gebruiksdoelVerblijfsobject>woonfunctie</bag_LVC:gebruiksdoelVerblijfsobject><bag_LVC:oppervlakteVerblijfsobject>72</bag_LVC:oppervlakteVerblijfsobject><bag_LVC:verblijfsobjectStatus>Verblijfsobject in gebruik</bag_LVC:verblijfsobjectStatus><bag_LVC:tijdvakgeldigheid><bagtype:begindatumTijdvakGeldigheid>2018032600000000</bagtype:begindatumTijdvakGeldigheid><bagtype:einddatumTijdvakGeldigheid>2018040400000000</bagtype:einddatumTijdvakGeldigheid></bag_LVC:tijdvakgeldigheid><bag_LVC:inOnderzoek>N</bag_LVC:inOnderzoek><bag_LVC:bron><bagtype:documentdatum>20180326</bagtype:documentdatum><bagtype:documentnummer>BV05.00043-HLG</bagtype:documentnummer></bag_LVC:bron><bag_LVC:gerelateerdPand><bag_LVC:identificatie>1883100000010452</bag_LVC:identificatie></bag_LVC:gerelateerdPand></bag_LVC:Verblijfsobject>
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
}
