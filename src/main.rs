use std::collections::hash_map::HashMap;
use std::path::{Path, PathBuf};

fn main() -> std::io::Result<()> {
    use clap::{App, Arg, SubCommand};

    let app = App::new("bag-extract")
        .subcommand(
            SubCommand::with_name("generate")
                .about("extract postcode <-> location data from inspireadressen")
                .arg(
                    Arg::with_name("SOURCE_DIR")
                        .short("s")
                        .long("source")
                        .help("Path of the input directory")
                        .default_value("/home/folkertdev/Downloads/inspire/foo"),
                ),
        )
        .subcommand(
            SubCommand::with_name("lookup")
                .about("lookup postcodes close to a coordinate")
                .arg(
                    Arg::with_name("SOURCE_DIR")
                        .short("s")
                        .long("source")
                        .help("Path of the input directory")
                        .default_value("/home/folkertdev/Downloads/inspire/foo"),
                ),
        );

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("generate") {
        let base_dir = matches.value_of("SOURCE_DIR").unwrap();

        parse_and_persist(&PathBuf::from(base_dir))
    } else if let Some(matches) = matches.subcommand_matches("lookup") {
        let base_dir = matches.value_of("SOURCE_DIR").unwrap();

        let postcodes = extract(&PathBuf::from(base_dir), POINTS, 50.0)?;

        for postcode in postcodes {
            println!("{}", postcode.to_string());
        }

        Ok(())
    } else {
        unreachable!()
    }
}

extern crate bagextract;

use bagextract::*;

use bounding_box::{BoundingBox, Point};
use memory_mapped_slice::MemoryMappedSlice;
use postcode::Postcode;

fn extract(base_path: &Path, points: &[Point], radius: f32) -> std::io::Result<Vec<Postcode>> {
    let bounding_boxes = BoundingBoxes::from_file(base_path.with_file_name("postcodes.bin"))?;
    let postcode_points = Points::from_files(
        base_path.with_file_name("points.bin"),
        base_path.with_file_name("slices.bin"),
    )?;

    let target = work(bounding_boxes, postcode_points, points, radius);

    Ok(target)
}

/// Parse the VBO and NUM zip files, extract the relevant data, and persist it to disk
fn parse_and_persist(base_path: &Path) -> std::io::Result<()> {
    let verblijfsobjecten_path = base_path.with_file_name("9999VBO08102021.zip");
    let nummeraanduidingen_path = base_path.with_file_name("9999NUM08102021.zip");

    let mut bounding_boxes = vec![bounding_box::INFINITE; 1 << 24];
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

                        bounding_boxes[index].extend_with(point);
                        points_per_postcode[index].push(point);
                    }
                }
            }
        }
        _ => panic!(),
    }

    BoundingBoxes::create_file(base_path.with_file_name("postcodes.bin"), &bounding_boxes)?;

    Points::create_files(
        base_path.with_file_name("points.bin"),
        base_path.with_file_name("slices.bin"),
        points_per_postcode,
    )?;

    Ok(())
}

fn work(
    bounding_boxes: BoundingBoxes,
    points_per_postcode: Points,
    input: &[Point],
    radius: f32,
) -> Vec<Postcode> {
    let target_bounding_boxes: Vec<_> = input
        .iter()
        .map(|point| BoundingBox::around_point(*point, radius))
        .collect();

    let bounding_boxes = bounding_boxes.bounding_boxes.as_slice();

    // evaluate in parallel if there are many points
    let mut result: Vec<_> = if input.len() <= 64 {
        bounding_boxes
            .iter()
            .enumerate()
            .map(|(i, bounding_box)| {
                check_against_address_locations(
                    i,
                    bounding_box,
                    input,
                    &target_bounding_boxes,
                    radius,
                    &points_per_postcode,
                )
            })
            .flatten()
            .collect()
    } else {
        use rayon::prelude::*;

        bounding_boxes
            .into_par_iter()
            .enumerate()
            .map(|(i, bounding_box)| {
                check_against_address_locations(
                    i,
                    bounding_box,
                    input,
                    &target_bounding_boxes,
                    radius,
                    &points_per_postcode,
                )
            })
            .flatten()
            .collect()
    };

    result.sort();

    result
}

fn check_against_address_locations(
    i: usize,
    bounding_box: &BoundingBox,
    input: &[Point],
    target_bounding_boxes: &[BoundingBox],
    radius: f32,
    points_per_postcode: &Points,
) -> Option<Postcode> {
    for (point, needle) in input.iter().zip(target_bounding_boxes.iter()) {
        if bounding_box.intersects_with(*needle) {
            let postcode = Postcode::from_u32(i as u32);

            let points = points_per_postcode.for_postcode(postcode);

            let close_enough = points.iter().any(|p| {
                use geoutils::Location;
                let distance = geoutils::Distance::from_meters(radius);
                let a = Location::new(p.x, p.y);
                let b = Location::new(point.x, point.y);

                a.is_in_circle(&b, distance).unwrap()
            });

            if close_enough {
                return Some(postcode);
            }
        }
    }

    None
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

struct BoundingBoxes {
    bounding_boxes: MemoryMappedSlice<BoundingBox>,
}

impl BoundingBoxes {
    fn from_file<P>(bin_path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index = Self {
            bounding_boxes: MemoryMappedSlice::from_file(bin_path)?,
        };

        Ok(index)
    }

    fn create_file<P>(bin_path: P, data: &[BoundingBox]) -> std::io::Result<()>
    where
        P: AsRef<Path>,
    {
        write_slice_to_file(bin_path, data)
    }

    fn for_postcode(&self, postcode: Postcode) -> BoundingBox {
        let index = postcode.as_u32() as usize;

        self.bounding_boxes.as_slice()[index]
    }

    fn postcodes_that_intersect_with(&self, needle: BoundingBox) -> Vec<Postcode> {
        let mut result = Vec::with_capacity(64);

        let it = self.bounding_boxes.as_slice().iter().enumerate();
        for (i, bounding_box) in it {
            if bounding_box.intersects_with(needle) {
                let postcode = Postcode::from_u32(i as u32);
                result.push(postcode);
            }
        }

        result
    }
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

    fn for_postcode(&self, postcode: Postcode) -> &[Point] {
        let slices = self.slices.as_slice();
        let points = self.points.as_slice();

        let index = postcode.as_u32() as usize;
        let (start, length) = slices[index];

        &points[start as usize..][..length as usize]
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

const MORE_POINTS: &[Point] = &[
    Point::new(5.67965488866478, 52.1405643787047),
    Point::new(5.58824751005124, 52.1424825685008),
    Point::new(6.69765391634653, 52.9535333196805),
    Point::new(5.58196224851979, 52.1397291700453),
    Point::new(5.58646413242308, 52.1454965669182),
    Point::new(5.69526770038764, 52.1485969543213),
    Point::new(5.59397927301206, 52.1359522864284),
    Point::new(5.60797523171513, 52.1748094047371),
    Point::new(5.62780379850877, 52.1499673367495),
    Point::new(5.61531544977061, 52.165092066236),
    Point::new(5.67965488866478, 52.1405643787047),
    Point::new(5.58824751005124, 52.1424825685008),
    Point::new(6.69765391634653, 52.9535333196805),
    Point::new(5.58196224851979, 52.1397291700453),
    Point::new(5.58646413242308, 52.1454965669182),
    Point::new(5.69526770038764, 52.1485969543213),
    Point::new(5.59397927301206, 52.1359522864284),
    Point::new(5.60797523171513, 52.1748094047371),
    Point::new(5.62780379850877, 52.1499673367495),
    Point::new(5.61531544977061, 52.165092066236),
    Point::new(4.75043861004033, 52.2660314081271),
    Point::new(6.39971294118668, 51.9201243809166),
    Point::new(6.46930090368336, 51.8867760218735),
    Point::new(4.74537091799729, 52.6322889012989),
    Point::new(6.40750223896834, 51.90077531953),
    Point::new(6.54806454999534, 52.0703497746592),
    Point::new(6.61128490091496, 52.1296296789618),
    Point::new(5.3758568966009, 51.9031114241937),
    Point::new(4.75043861004033, 52.2660314081271),
    Point::new(6.39971294118668, 51.9201243809166),
    Point::new(6.46930090368336, 51.8867760218735),
    Point::new(4.74537091799729, 52.6322889012989),
    Point::new(6.40750223896834, 51.90077531953),
    Point::new(6.54806454999534, 52.0703497746592),
    Point::new(6.61128490091496, 52.1296296789618),
    Point::new(5.3758568966009, 51.9031114241937),
    Point::new(4.43880552703027, 51.5135759144196),
    Point::new(4.37259199039176, 51.4931533285736),
    Point::new(4.34405636184264, 51.5321333641461),
    Point::new(4.48849790929674, 51.5460307075546),
    Point::new(5.22647034835451, 51.9553048864381),
    Point::new(5.22078729868626, 51.9618044590545),
    Point::new(4.95827553487323, 51.5171062797425),
    Point::new(3.82540879844178, 51.4989831750745),
    Point::new(5.22312217018264, 51.955782574798),
    Point::new(4.89733153491227, 52.3747207614601),
    Point::new(4.43880552703027, 51.5135759144196),
    Point::new(4.37259199039176, 51.4931533285736),
    Point::new(4.34405636184264, 51.5321333641461),
    Point::new(4.48849790929674, 51.5460307075546),
    Point::new(5.22647034835451, 51.9553048864381),
    Point::new(5.22078729868626, 51.9618044590545),
    Point::new(4.95827553487323, 51.5171062797425),
    Point::new(3.82540879844178, 51.4989831750745),
    Point::new(5.22312217018264, 51.955782574798),
    Point::new(4.89733153491227, 52.3747207614601),
    Point::new(6.15951072725779, 53.4780848844606),
    Point::new(6.15269767332427, 53.4879220760491),
    Point::new(6.17782463216233, 53.4788376836731),
    Point::new(6.6659394247382, 52.3548651990337),
    Point::new(5.90375161403887, 51.9812708553279),
    Point::new(5.91258225566612, 51.9816154027334),
    Point::new(4.9006543556654, 52.3756961455129),
    Point::new(4.77851372406812, 51.5947264653114),
    Point::new(4.89377254986862, 52.3593459728095),
    Point::new(5.90940505555768, 51.9683680361103),
    Point::new(6.15951072725779, 53.4780848844606),
    Point::new(6.15269767332427, 53.4879220760491),
    Point::new(6.17782463216233, 53.4788376836731),
    Point::new(6.6659394247382, 52.3548651990337),
    Point::new(5.90375161403887, 51.9812708553279),
    Point::new(5.91258225566612, 51.9816154027334),
    Point::new(4.9006543556654, 52.3756961455129),
    Point::new(4.77851372406812, 51.5947264653114),
    Point::new(4.89377254986862, 52.3593459728095),
    Point::new(5.90940505555768, 51.9683680361103),
    Point::new(4.89349785374828, 52.3774371006303),
    Point::new(4.88442544100964, 52.364169007212),
    Point::new(4.90031091474093, 52.3735376855662),
    Point::new(4.92902396945904, 52.3631546417713),
    Point::new(4.93573985466178, 52.4000208946646),
    Point::new(4.88856934040211, 52.3690668664935),
    Point::new(4.88347841305744, 52.3648210857948),
    Point::new(4.90033550690364, 52.3686215106702),
    Point::new(4.93875169183112, 52.372458199188),
    Point::new(4.90354703135262, 52.3514860928239),
    Point::new(4.89349785374828, 52.3774371006303),
    Point::new(4.88442544100964, 52.364169007212),
    Point::new(4.90031091474093, 52.3735376855662),
    Point::new(4.92902396945904, 52.3631546417713),
    Point::new(4.93573985466178, 52.4000208946646),
    Point::new(4.88856934040211, 52.3690668664935),
    Point::new(4.88347841305744, 52.3648210857948),
    Point::new(4.90033550690364, 52.3686215106702),
    Point::new(4.93875169183112, 52.372458199188),
    Point::new(4.90354703135262, 52.3514860928239),
    Point::new(4.9143904967743, 51.5966343198703),
    Point::new(4.93651487135396, 51.5454146300038),
    Point::new(5.93509125724059, 53.2147861700208),
    Point::new(6.11938290137999, 53.1815920721668),
    Point::new(5.93820880327122, 53.2098136051886),
    Point::new(5.48813368120719, 51.9580298901076),
    Point::new(4.9143904967743, 51.5966343198703),
    Point::new(4.93651487135396, 51.5454146300038),
    Point::new(5.93509125724059, 53.2147861700208),
    Point::new(6.11938290137999, 53.1815920721668),
    Point::new(5.93820880327122, 53.2098136051886),
    Point::new(5.48813368120719, 51.9580298901076),
    Point::new(5.68980113215359, 52.1870346531975),
    Point::new(5.59896028654163, 52.1753170399278),
    Point::new(5.50838406022866, 52.1213352274164),
    Point::new(5.65062532269601, 52.2032526676245),
    Point::new(5.72596522654479, 52.22778203164),
    Point::new(5.49877983325176, 52.183251003256),
    Point::new(5.83931366445763, 51.1622768181272),
    Point::new(5.64415821020799, 52.1930987395766),
    Point::new(5.68469042346532, 51.9689336365007),
    Point::new(5.65734268944191, 52.1805972102343),
    Point::new(5.68980113215359, 52.1870346531975),
    Point::new(5.59896028654163, 52.1753170399278),
    Point::new(5.50838406022866, 52.1213352274164),
    Point::new(5.65062532269601, 52.2032526676245),
    Point::new(5.72596522654479, 52.22778203164),
    Point::new(5.49877983325176, 52.183251003256),
    Point::new(5.83931366445763, 51.1622768181272),
    Point::new(5.64415821020799, 52.1930987395766),
    Point::new(5.68469042346532, 51.9689336365007),
    Point::new(5.65734268944191, 52.1805972102343),
    Point::new(4.09254655513546, 51.2839185233738),
    Point::new(5.99372343576583, 52.934683731571),
    Point::new(4.93508416007958, 52.4095632526865),
    Point::new(4.91004833813961, 52.3897821169503),
    Point::new(5.9219190320784, 51.9834661478072),
    Point::new(5.91305387241254, 51.9829541388098),
    Point::new(4.9772792357077, 52.2950157350758),
    Point::new(4.90050593001279, 52.3758393387278),
    Point::new(6.31724563444831, 52.0403453535578),
    Point::new(4.09254655513546, 51.2839185233738),
    Point::new(5.99372343576583, 52.934683731571),
    Point::new(4.93508416007958, 52.4095632526865),
    Point::new(4.91004833813961, 52.3897821169503),
    Point::new(5.9219190320784, 51.9834661478072),
    Point::new(5.91305387241254, 51.9829541388098),
    Point::new(4.9772792357077, 52.2950157350758),
    Point::new(4.90050593001279, 52.3758393387278),
    Point::new(6.31724563444831, 52.0403453535578),
];
