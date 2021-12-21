use std::collections::hash_map::HashMap;
use std::path::{Path, PathBuf};

fn main() -> std::io::Result<()> {
    // let f = std::fs::File::open("/home/folkertdev/tg/pect/bagextract/single.xml").unwrap();
    // let mut reader = BufReader::new(f);
    // let all: Wrapper = quick_xml::de::from_reader(reader).unwrap();

    extract()
}

mod bounding_box;
mod parse_num;
mod parse_vbo;
mod postcode;

use bounding_box::{BoundingBox, Point};
use postcode::CompactPostcode;

fn extract() -> std::io::Result<()> {
    if false {
        let verblijfsobjecten_path =
            PathBuf::from("/home/folkertdev/Downloads/inspire/9999VBO08102021.zip");
        let nummeraanduidingen_path =
            PathBuf::from("/home/folkertdev/Downloads/inspire/9999NUM08102021.zip");

        let mut store = Vec::from_iter(std::iter::repeat(bounding_box::INFINITE).take(1 << 24));
        let mut points_per_postcode = Vec::from_iter(std::iter::repeat(Vec::new()).take(1 << 24));

        let ns = parse_num::parse(&nummeraanduidingen_path);
        let vs = parse_vbo::parse(&verblijfsobjecten_path);

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

                /*
                for (id, geopunt) in it {
                    match map.get(&id) {
                        None => {}
                        Some(postcode) => {
                            let index = postcode.as_u32() as usize;
                            let point =
                                bounding_box::Point::from_rijksdriehoek(geopunt.x, geopunt.y);
                            store[index].extend_with(point);
                        }
                    }
                }
                */

                for (id, geopunt) in it {
                    match map.get(&id) {
                        None => {}
                        Some(postcode) => {
                            let index = postcode.as_u32() as usize;
                            let point =
                                bounding_box::Point::from_rijksdriehoek(geopunt.x, geopunt.y);
                            points_per_postcode[index].push(point);
                        }
                    }
                }
            }
            _ => panic!(),
        }

        // write_slice_to_file("/home/folkertdev/Downloads/inspire/postcodes.bin", &store)?;

        let mut points = Vec::with_capacity(600_000);
        let mut slices = Vec::with_capacity(1 << 24);

        for points_with_postcode in points_per_postcode.into_iter() {
            // let postcode = CompactPostcode::from_u32(i as u32);

            let start = points.len();
            let length = points_with_postcode.len();

            points.extend(points_with_postcode);

            slices.push((start as u32, length as u32));
        }

        write_slice_to_file("/home/folkertdev/Downloads/inspire/points.bin", &points)?;
        write_slice_to_file("/home/folkertdev/Downloads/inspire/slices.bin", &slices)?;
    }

    let store = MemMappedStore::from_files("/home/folkertdev/Downloads/inspire/postcodes.bin")?;
    let points_store = MemMappedPoints::from_files(
        "/home/folkertdev/Downloads/inspire/points.bin",
        "/home/folkertdev/Downloads/inspire/slices.bin",
    )?;

    let target = work(store, points_store, POINTS);

    let pretty: Vec<_> = target.iter().map(|x| x.to_string()).collect();

    dbg!(pretty);

    Ok(())
}

fn work(
    store: MemMappedStore,
    points_store: MemMappedPoints,
    input: &[Point],
) -> Vec<CompactPostcode> {
    let mut result = Vec::new();
    let radius = 50.0;

    for point in input {
        let bb = BoundingBox::around_point(*point, radius);

        let mut target = store.postcodes_that_intersect_with(bb);

        target.retain(|postcode| {
            let points = points_store.for_postcode(*postcode);

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

struct MemMappedStore {
    bounding_boxes: memmap::Mmap,
}

impl MemMappedStore {
    fn from_files<P>(bin_path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let bounding_boxes_file = std::fs::File::open(bin_path)?;

        let index = Self {
            bounding_boxes: unsafe { memmap::Mmap::map(&bounding_boxes_file)? },
        };

        Ok(index)
    }

    /// Cast a slice of bytes to a slice of `T`.
    /// Extracted into a function to constrain the lifetime of the result
    fn cast_slice<T>(slice: &[u8]) -> &[T] {
        let element_width = slice.len() / std::mem::size_of::<T>();
        let ptr = slice.as_ptr();

        unsafe { std::slice::from_raw_parts(ptr as *const _, element_width) }
    }

    fn iter(&self) -> impl Iterator<Item = (CompactPostcode, &BoundingBox)> {
        let bounding_boxes = Self::cast_slice::<BoundingBox>(&self.bounding_boxes);

        bounding_boxes
            .iter()
            .enumerate()
            .filter(|(_, bb)| !bb.is_infinite())
            .map(|(i, bb)| (CompactPostcode::from_u32(i as u32), bb))
    }

    fn for_postcode(&self, postcode: CompactPostcode) -> BoundingBox {
        let index = postcode.as_u32() as usize;

        let bounding_boxes = Self::cast_slice::<BoundingBox>(&self.bounding_boxes);

        bounding_boxes[index]
    }

    fn postcodes_that_intersect_with(&self, needle: BoundingBox) -> Vec<CompactPostcode> {
        let mut result = Vec::with_capacity(64);

        for (postcode, bounding_box) in self.iter() {
            if bounding_box.intersects_with(needle) {
                result.push(postcode);
            }
        }

        result
    }
}

struct MemMappedPoints {
    points: memmap::Mmap,
    slices: memmap::Mmap,
}

impl MemMappedPoints {
    fn from_files<P>(points_path: P, slices_path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let points_file = std::fs::File::open(points_path)?;
        let slices_file = std::fs::File::open(slices_path)?;

        let index = Self {
            points: unsafe { memmap::Mmap::map(&points_file)? },
            slices: unsafe { memmap::Mmap::map(&slices_file)? },
        };

        Ok(index)
    }

    /// Cast a slice of bytes to a slice of `T`.
    /// Extracted into a function to constrain the lifetime of the result
    fn cast_slice<T>(slice: &[u8]) -> &[T] {
        let element_width = slice.len() / std::mem::size_of::<T>();
        let ptr = slice.as_ptr();

        unsafe { std::slice::from_raw_parts(ptr as *const _, element_width) }
    }

    fn for_postcode(&self, postcode: CompactPostcode) -> &[Point] {
        let slices = Self::cast_slice::<(u32, u32)>(&self.slices);
        let points = Self::cast_slice::<Point>(&self.points);

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
