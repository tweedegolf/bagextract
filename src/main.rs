use std::collections::hash_map::HashMap;
use std::path::{Path, PathBuf};

fn main() -> std::io::Result<()> {
    extract()
}

extern crate bagextract;

use bagextract::*;

use bounding_box::{BoundingBox, Point};
use memory_mapped_slice::MemoryMappedSlice;
use postcode::CompactPostcode;

fn extract() -> std::io::Result<()> {
    if true {
        parse_and_persist()?;
    }

    let bounding_boxes =
        BoundingBoxes::from_file("/home/folkertdev/Downloads/inspire/postcodes.bin")?;
    let postcode_points = Points::from_files(
        "/home/folkertdev/Downloads/inspire/points.bin",
        "/home/folkertdev/Downloads/inspire/slices.bin",
    )?;

    dbg!(
        bounding_boxes.bounding_boxes.len(),
        postcode_points.points.len()
    );

    let target = work(bounding_boxes, postcode_points, POINTS, 50.0);

    let pretty: Vec<_> = target.iter().map(|x| x.to_string()).collect();

    dbg!(pretty);

    Ok(())
}

/// Parse the VBO and NUM zip files, extract the relevant data, and persist it to disk
fn parse_and_persist() -> std::io::Result<()> {
    let verblijfsobjecten_path =
        PathBuf::from("/home/folkertdev/Downloads/inspire/9999VBO08102021.zip");
    let nummeraanduidingen_path =
        PathBuf::from("/home/folkertdev/Downloads/inspire/9999NUM08102021.zip");

    let mut bounding_boxes =
        Vec::from_iter(std::iter::repeat(bounding_box::INFINITE).take(1 << 24));
    let mut points_per_postcode = Vec::from_iter(std::iter::repeat(Vec::new()).take(1 << 24));

    let ns_handle = std::thread::spawn(move || parse_num::parse(&nummeraanduidingen_path));
    let vs = parse_vbo::parse(&verblijfsobjecten_path);

    let ns = ns_handle.join().unwrap();

    match (vs, ns) {
        (Ok(verblijfsobjecten), Ok(nummeraanduidingen)) => {
            let it = nummeraanduidingen
                .identificatie
                .into_iter()
                .zip(nummeraanduidingen.postcodes.into_iter());
            let map: HashMap<u64, CompactPostcode> = it.collect();

            let it = verblijfsobjecten
                .postcode_id
                .into_iter()
                .zip(verblijfsobjecten.geopunten.into_iter());

            for (id, geopunt) in it {
                match map.get(&id) {
                    None => {}
                    Some(postcode) => {
                        let index = postcode.as_u32() as usize;
                        let point = bounding_box::Point::from_rijksdriehoek(geopunt.x, geopunt.y);

                        bounding_boxes[index].extend_with(point);
                        points_per_postcode[index].push(point);
                    }
                }
            }
        }
        _ => panic!(),
    }

    BoundingBoxes::create_file(
        "/home/folkertdev/Downloads/inspire/postcodes.bin",
        &bounding_boxes,
    )?;

    Points::create_files(
        "/home/folkertdev/Downloads/inspire/points.bin",
        "/home/folkertdev/Downloads/inspire/slices.bin",
        points_per_postcode,
    )?;

    Ok(())
}

fn work(
    bounding_boxes: BoundingBoxes,
    points_per_postcode: Points,
    input: &[Point],
    radius: f32,
) -> Vec<CompactPostcode> {
    let mut result = Vec::new();

    for point in input {
        let bb = BoundingBox::around_point(*point, radius);

        let mut target = bounding_boxes.postcodes_that_intersect_with(bb);

        target.retain(|postcode| {
            let points = points_per_postcode.for_postcode(*postcode);

            points.iter().any(|p| {
                use geoutils::Location;
                let a = Location::new(p.x, p.y);
                let b = Location::new(point.x, point.y);

                a.haversine_distance_to(&b).meters() <= radius as f64
            })
        });

        result.extend(target);
    }

    result.sort();
    result.dedup();

    result
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

    fn for_postcode(&self, postcode: CompactPostcode) -> BoundingBox {
        let index = postcode.as_u32() as usize;

        self.bounding_boxes.as_slice()[index]
    }

    fn postcodes_that_intersect_with(&self, needle: BoundingBox) -> Vec<CompactPostcode> {
        let mut result = Vec::with_capacity(64);

        let it = self.bounding_boxes.as_slice().iter().enumerate();
        for (i, bounding_box) in it {
            if bounding_box.intersects_with(needle) {
                let postcode = CompactPostcode::from_u32(i as u32);
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
        let mut points = Vec::with_capacity(600_000);
        let mut slices = Vec::with_capacity(1 << 24);

        for points_with_postcode in points_per_postcode.iter() {
            // let postcode = CompactPostcode::from_u32(i as u32);

            let start = points.len();
            let length = points_with_postcode.len();

            points.extend(points_with_postcode.iter().copied());

            slices.push((start as u32, length as u32));
        }

        write_slice_to_file(points_path, &points)?;
        write_slice_to_file(slices_path, &slices)?;

        Ok(())
    }

    fn for_postcode(&self, postcode: CompactPostcode) -> &[Point] {
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
