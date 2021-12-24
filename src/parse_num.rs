// Parse Nummeraanduiding zip file

use std::io::BufReader;
use std::path::Path;

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

        if file.name().ends_with('/') {
            println!("Entry {} is a directory with name \"{}\"", i, file.name());
        } else {
            println!(
                "Entry {} is a file with name \"{}\" ({} bytes)",
                i,
                file.name(),
                file.size()
            );

            let reader = BufReader::new(file);
            // process_xml(&mut result, reader)?;
            parse_manual_step(reader, &mut result).unwrap();
        }
    }

    Ok(result)
}

#[derive(Debug)]
pub struct Nummeraanduiding {
    identificatie: u64,
    postcode: Option<CompactPostcode>,
}

pub fn parse_manual_str(input: &str) -> Option<Postcodes> {
    let mut result = Postcodes {
        identificatie: Vec::with_capacity(1024),
        postcodes: Vec::with_capacity(1024),
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
                if let b"bag_LVC:Nummeraanduiding" = e.name() {
                    let aanduiding = parse_manual_help(&mut reader, &mut buf)?;
                    if let Some(postcode) = aanduiding.postcode {
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
                b"bag_LVC:identificatie" => state = State::Identificatie,
                b"bag_LVC:postcode" => state = State::Postcode,
                _ => (),
            },
            Ok(Event::End(ref e)) => {
                if let b"bag_LVC:Nummeraanduiding" = e.name() {
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
                    postcode = Some(CompactPostcode::try_from(string).unwrap());
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

    #[test]
    fn parse_nummeraanduiding_manual() {
        let input = r#"
            <bag_LVC:Nummeraanduiding><bag_LVC:identificatie>0000200000057534</bag_LVC:identificatie><bag_LVC:aanduidingRecordInactief>N</bag_LVC:aanduidingRecordInactief><bag_LVC:aanduidingRecordCorrectie>0</bag_LVC:aanduidingRecordCorrectie><bag_LVC:huisnummer>32</bag_LVC:huisnummer><bag_LVC:officieel>N</bag_LVC:officieel><bag_LVC:huisletter>A</bag_LVC:huisletter><bag_LVC:postcode>6131BE</bag_LVC:postcode><bag_LVC:tijdvakgeldigheid><bagtype:begindatumTijdvakGeldigheid>2018032600000000</bagtype:begindatumTijdvakGeldigheid><bagtype:einddatumTijdvakGeldigheid>2018040400000000</bagtype:einddatumTijdvakGeldigheid></bag_LVC:tijdvakgeldigheid><bag_LVC:inOnderzoek>N</bag_LVC:inOnderzoek><bag_LVC:typeAdresseerbaarObject>Verblijfsobject</bag_LVC:typeAdresseerbaarObject><bag_LVC:bron><bagtype:documentdatum>20180326</bagtype:documentdatum><bagtype:documentnummer>BV05.00043-HLG</bagtype:documentnummer></bag_LVC:bron><bag_LVC:nummeraanduidingStatus>Naamgeving uitgegeven</bag_LVC:nummeraanduidingStatus><bag_LVC:gerelateerdeOpenbareRuimte><bag_LVC:identificatie>1883300000001522</bag_LVC:identificatie></bag_LVC:gerelateerdeOpenbareRuimte></bag_LVC:Nummeraanduiding>
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

    #[test]
    fn compact_postcode() {
        let input = r#"<Foo>9999XX</Foo>"#;

        let object: CompactPostcode = quick_xml::de::from_str(input).unwrap();

        dbg!(&object);
    }
}
