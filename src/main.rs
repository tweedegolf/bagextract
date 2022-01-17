use std::collections::hash_map::HashMap;
use std::path::{Path, PathBuf};

extern crate bagextract;

use bagextract::*;

use memory_mapped_slice::MemoryMappedSlice;
use point::Point;
use postcode::Postcode;

fn main() -> std::io::Result<()> {
    use clap::{App, Arg, SubCommand};

    let app = App::new("bag-extract").subcommand(
        SubCommand::with_name("generate")
            .about("extract postcode <-> location data from inspireadressen")
            .arg(
                Arg::with_name("SOURCE_DIR")
                    .short("s")
                    .long("source")
                    .help("Path of the input directory")
                    .default_value("/home/folkertdev/Downloads/inspire"),
            )
            .arg(
                Arg::with_name("HOST")
                    .long("host")
                    .help("database host")
                    .default_value("localhost"),
            )
            .arg(
                Arg::with_name("USER")
                    .long("user")
                    .help("database user")
                    .default_value("tgbag"),
            )
            .arg(
                Arg::with_name("PASSWORD")
                    .long("password")
                    .help("database password")
                    .default_value("tgbag"),
            )
            .arg(
                Arg::with_name("DBNAME")
                    .long("dbname")
                    .help("database dbname")
                    .default_value("bagextract"),
            ),
    );

    // format!("host=localhost user=tgbag password=tgbag dbname=bagextract",

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("generate") {
        let base_dir = matches.value_of("SOURCE_DIR").unwrap();

        let db_credentials = DbCredentials {
            host: matches.value_of("HOST").unwrap().to_string(),
            user: matches.value_of("USER").unwrap().to_string(),
            password: matches.value_of("PASSWORD").unwrap().to_string(),
            dbname: matches.value_of("DBNAME").unwrap().to_string(),
        };

        let debug = true;
        if debug {
            parse_and_db_debug(&PathBuf::from(base_dir), &db_credentials)
        } else {
            parse_and_db(&PathBuf::from(base_dir), &db_credentials)
        }
    } else {
        unreachable!()
    }
}

fn parse_points_per_postcode(base_path: &Path) -> std::io::Result<Vec<Vec<Point>>> {
    // let verblijfsobjecten_path = base_path.with_file_name("9999VBO08102021.zip");
    // let nummeraanduidingen_path = base_path.with_file_name("9999NUM08102021.zip");
    let verblijfsobjecten_path = base_path.join("vbo.zip");
    let nummeraanduidingen_path = base_path.join("num.zip");

    let mut points_per_postcode = vec![Vec::new(); 1 << 24];

    let vs = parse_vbo::parse(&verblijfsobjecten_path);
    let ns = parse_num::parse(&nummeraanduidingen_path);

    match (vs, ns) {
        (Ok(verblijfsobjecten), Ok(nummeraanduidingen)) => {
            let it = nummeraanduidingen
                .identificatie
                .into_iter()
                .zip(nummeraanduidingen.postcodes.into_iter());
            let map: HashMap<u64, Postcode> = it.collect();

            let it = verblijfsobjecten
                .postcode_id
                .into_iter()
                .zip(verblijfsobjecten.points.into_iter());

            for (id, point) in it {
                match map.get(&id) {
                    None => {}
                    Some(postcode) => {
                        let index = postcode.as_u32() as usize;

                        points_per_postcode[index].push(point);
                    }
                }
            }
        }
        (Err(e), _) => panic!("verblijfsobjecten {:?}", e),
        (_, Err(e)) => panic!("nummeraanduidingen {:?}", e),
    }

    Ok(points_per_postcode)
}

fn parse_and_db(base_path: &Path, db_credentials: &DbCredentials) -> std::io::Result<()> {
    let points_per_postcode = parse_points_per_postcode(base_path)?;
    let it = points_per_postcode
        .iter()
        .enumerate()
        .map(|(i, points)| (Postcode::from_index(i), points.as_slice()))
        .skip(Postcode::MIN.as_index());

    populate_database(db_credentials, it)?;

    Ok(())
}

/// Parse the VBO and NUM zip files, extract the relevant data, and persist it to disk
fn parse_and_db_debug(base_path: &Path, db_credentials: &DbCredentials) -> std::io::Result<()> {
    if false {
        let points_per_postcode = parse_points_per_postcode(base_path)?;

        Points::create_files(
            base_path.join("points-28992.bin"),
            base_path.join("slices-28992.bin"),
            points_per_postcode,
        )?;
    }

    let points_per_postcode = Points::from_files(
        base_path.join("points-28992.bin"),
        base_path.join("slices-28992.bin"),
    )?;

    let it = points_per_postcode.iterate_postcodes();

    populate_database(db_credentials, it)?;

    Ok(())
}

struct DbCredentials {
    host: String,
    user: String,
    password: String,
    dbname: String,
}

fn populate_database<'a, I>(db_credentials: &DbCredentials, data: I) -> std::io::Result<()>
where
    I: Iterator<Item = (Postcode, &'a [Point])>,
{
    use postgres::{Client, NoTls};

    let arguments = &format!(
        "host={} user={} password={} dbname={}",
        db_credentials.host, db_credentials.user, db_credentials.password, db_credentials.dbname
    );

    let mut client = Client::connect(arguments, NoTls).unwrap();

    println!("Inserting data into adressen_28992");

    let mut writer = client.copy_in("COPY adressen_28992 FROM stdin").unwrap();
    for (postcode, points) in data {
        if postcode > Postcode::MAX {
            break;
        }

        for point in points.iter() {
            {
                use std::io::Write;

                writeln!(writer, "POINT({} {})\t{}", point.x, point.y, postcode)?;
            }
        }
    }

    let rows_written = writer.finish().unwrap();

    println!("Done inserting data, inserted {} rows", rows_written);

    Ok(())
}

/// Turn a `&[T]` into a `&[u8]` and write it to a file. Clearly that only works
/// if a value of type `T` is fully represented by its bytes (e.g. no heap allocations)
fn write_slice_to_file<P, T: Copy>(path: P, slice: &[T]) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let ptr = slice.as_ptr();
    let byte_width = slice.len() * std::mem::size_of::<T>();

    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(ptr as *const _, byte_width) };

    std::fs::write(path, bytes)
}

struct Points {
    points: MemoryMappedSlice<Point>,
    slices: MemoryMappedSlice<(u32, u32)>,
}

impl Points {
    fn from_files<P>(points_path: P, slices_path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index = Self {
            points: MemoryMappedSlice::from_file(points_path)?,
            slices: MemoryMappedSlice::from_file(slices_path)?,
        };

        Ok(index)
    }

    fn create_files<P>(
        points_path: P,
        slices_path: P,
        points_per_postcode: Vec<Vec<Point>>,
    ) -> std::io::Result<()>
    where
        P: AsRef<Path>,
    {
        let mut points = Vec::with_capacity(700_000);
        let mut slices = Vec::with_capacity(1 << 24);

        for points_with_postcode in points_per_postcode.iter() {
            let start = points.len();
            let length = points_with_postcode.len();

            points.extend(points_with_postcode.iter().copied());

            slices.push((start as u32, length as u32));
        }

        write_slice_to_file(points_path, &points)?;
        write_slice_to_file(slices_path, &slices)?;

        Ok(())
    }

    fn iterate_postcodes(&self) -> impl Iterator<Item = (Postcode, &[Point])> {
        let slices = self.slices.as_slice();
        let points = self.points.as_slice();

        (0..(1 << 24)).map(|index| {
            let postcode = Postcode::from_index(index);

            let (start, length) = slices[index];

            (postcode, &points[start as usize..][..length as usize])
        })
    }
}

const POINTS: &[Point] = &[
    Point::new(5.11007074917847, 52.062321384871),
    Point::new(4.7464139321804, 51.6071932738763),
    Point::new(4.86228629544573, 52.3053347047553),
    Point::new(4.03001520963129, 51.3487238241373),
    Point::new(4.8786874468356, 52.2992079812286),
    Point::new(5.82994166739256, 51.804506206861),
    Point::new(4.72007507606698, 51.5468387432124),
    Point::new(5.30626765415776, 52.162948264751),
    Point::new(5.11007074917847, 52.062321384871),
    Point::new(4.7464139321804, 51.6071932738763),
    Point::new(4.86228629544573, 52.3053347047553),
    Point::new(4.03001520963129, 51.3487238241373),
    Point::new(4.8786874468356, 52.2992079812286),
    Point::new(5.82994166739256, 51.804506206861),
    Point::new(4.72007507606698, 51.5468387432124),
    Point::new(5.30626765415776, 52.162948264751),
];

#[cfg(test)]
mod dbtest {
    use postgres::{Client, NoTls};

    fn test_helper(points: &[(f64, f64)]) -> Vec<String> {
        let mut client = Client::connect(
            "host=localhost user=tgbag password=tgbag dbname=bagextract",
            NoTls,
        )
        .unwrap();

        let mut buf = String::new();
        buf.push_str("SRID=4326;MULTIPOINT(");
        let mut it = points.iter().peekable();
        while let Some((x, y)) = it.next() {
            use std::fmt::Write;
            write!(buf, "{} {}", *x as f32, *y as f32).unwrap();

            if it.peek().is_some() {
                buf.push(',');
            }
        }
        buf.push(')');

        let mut result = Vec::new();

        for (x, y) in points {
            let query = "SELECT postcode FROM adressen WHERE ST_DWithin(ST_POINT($1, $2)::geography, point::geography, 50, false)";
            for row in client.query(query, &[x, y]).unwrap() {
                let postcode: &str = row.get(0);

                // println!("\"{}\",", postcode);
                result.push(postcode.to_string());
            }
        }

        result.sort();
        result.dedup();

        result
    }

    #[test]
    fn foo() {
        let points = &[
            (6.47821242976357, 51.9381148436214),
            (5.05615993640658, 52.6429354790004),
            (6.24815916803166, 51.8713990299342),
            (4.89957348912126, 52.3724920772592),
            (4.86766732715532, 52.3609509416471),
            (4.85655672329772, 52.3644158693014),
            (4.88601772166714, 52.3622793475881),
            (4.8735250602509, 52.3862949084857),
            (4.82016343709478, 52.311691604278),
            (4.87981345683438, 52.3733257971681),
            (6.47821242976357, 51.9381148436214),
            (5.05615993640658, 52.6429354790004),
            (6.24815916803166, 51.8713990299342),
            (4.89957348912126, 52.3724920772592),
            (4.86766732715532, 52.3609509416471),
            (4.85655672329772, 52.3644158693014),
            (4.88601772166714, 52.3622793475881),
            (4.8735250602509, 52.3862949084857),
            (4.82016343709478, 52.311691604278),
            (4.87981345683438, 52.3733257971681),
        ];

        let result = test_helper(points);

        let expected = &[
            "1012BS", "1012BV", "1012CR", "1012CS", "1012CT", "1012DC", "1014DB", "1016KX",
            "1016KZ", "1016LL", "1016LM", "1016LT", "1016LV", "1016NE", "1016NG", "1016NH",
            "1016NX", "1016PC", "1017NP", "1017NR", "1017RA", "1017RB", "1017RD", "1017RL",
            "1017RM", "1017RS", "1017RT", "1054DW", "1054HW", "1054JC", "1054JD", "1054JH",
            "1054JJ", "1057DT", "1057DV", "1057EB", "1057EC", "1057VZ", "1182DB", "1621HE",
            "1621HP", "1621HR", "1621JC", "1621JK", "7041AV", "7041AW", "7041SR", "7041SX",
            "7051HR",
        ] as &[_];

        assert_eq!(expected, &result);
    }
}
