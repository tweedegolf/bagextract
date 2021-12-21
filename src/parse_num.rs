// Parse Nummeraanduiding zip file
use serde_derive::Deserialize;

use std::io::BufReader;
use std::path::Path;

use crate::parse_wrapper::Wrapper;
use crate::postcode::CompactPostcode;

#[derive(Debug, Default)]
pub struct Postcodes {
    pub identificatie: Vec<u64>,
    pub postcodes: Vec<CompactPostcode>,
}

impl Postcodes {
    fn push(&mut self, identificatie: u64, postcode: CompactPostcode) {
        self.identificatie.push(identificatie);
        self.postcodes.push(postcode);
    }
}

pub fn parse(path: &Path) -> std::io::Result<Postcodes> {
    let mut result = Postcodes::default();

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

fn process_xml<R: std::io::Read>(result: &mut Postcodes, reader: R) -> std::io::Result<()> {
    let reader = BufReader::new(reader);
    let all: Wrapper<Nummeraanduiding> = quick_xml::de::from_reader(reader).unwrap();

    let objects = all.objects;

    for object in objects {
        match object.postcode {
            None => {
                println!(
                    "skipping nummeraanduiding {}, it has no postcode",
                    object.identificatie
                );
            }
            Some(postcode) => {
                let postcode = CompactPostcode::try_from(postcode.as_str()).unwrap();
                result.push(object.identificatie, postcode);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename = "bag_LVC:Nummeraanduiding")]
struct Nummeraanduiding {
    identificatie: u64,
    postcode: Option<String>,
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

        let object: Nummeraanduiding = quick_xml::de::from_str(input).unwrap();

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

        let object: Wrapper<Nummeraanduiding> = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }

    #[test]
    fn parse_nummeraanduiding() {
        let input = r#"
            <bag_LVC:Nummeraanduiding><bag_LVC:identificatie>0000200000057534</bag_LVC:identificatie><bag_LVC:aanduidingRecordInactief>N</bag_LVC:aanduidingRecordInactief><bag_LVC:aanduidingRecordCorrectie>0</bag_LVC:aanduidingRecordCorrectie><bag_LVC:huisnummer>32</bag_LVC:huisnummer><bag_LVC:officieel>N</bag_LVC:officieel><bag_LVC:huisletter>A</bag_LVC:huisletter><bag_LVC:postcode>6131BE</bag_LVC:postcode><bag_LVC:tijdvakgeldigheid><bagtype:begindatumTijdvakGeldigheid>2018032600000000</bagtype:begindatumTijdvakGeldigheid><bagtype:einddatumTijdvakGeldigheid>2018040400000000</bagtype:einddatumTijdvakGeldigheid></bag_LVC:tijdvakgeldigheid><bag_LVC:inOnderzoek>N</bag_LVC:inOnderzoek><bag_LVC:typeAdresseerbaarObject>Verblijfsobject</bag_LVC:typeAdresseerbaarObject><bag_LVC:bron><bagtype:documentdatum>20180326</bagtype:documentdatum><bagtype:documentnummer>BV05.00043-HLG</bagtype:documentnummer></bag_LVC:bron><bag_LVC:nummeraanduidingStatus>Naamgeving uitgegeven</bag_LVC:nummeraanduidingStatus><bag_LVC:gerelateerdeOpenbareRuimte><bag_LVC:identificatie>1883300000001522</bag_LVC:identificatie></bag_LVC:gerelateerdeOpenbareRuimte></bag_LVC:Nummeraanduiding>
"#;

        let object: Nummeraanduiding = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }
}
