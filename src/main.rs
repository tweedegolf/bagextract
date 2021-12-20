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

use bounding_box::BoundingBox;
use postcode::CompactPostcode;

fn extract() -> std::io::Result<()> {
    if true {
        let verblijfsobjecten_path =
            PathBuf::from("/home/folkertdev/Downloads/inspire/9999VBO08102021.zip");
        let nummeraanduidingen_path =
            PathBuf::from("/home/folkertdev/Downloads/inspire/9999NUM08102021.zip");

        let mut store = Vec::from_iter(std::iter::repeat(bounding_box::INFINITE).take(1 << 22));

        match (
            parse_vbo::parse(&verblijfsobjecten_path),
            parse_num::parse(&nummeraanduidingen_path),
        ) {
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
                            let point =
                                bounding_box::Point::from_rijksdriehoek(geopunt.x, geopunt.y);
                            store[index].extend_with(point);
                        }
                    }
                }
            }
            _ => panic!(),
        }

        write_slice_to_file("/home/folkertdev/Downloads/inspire/postcodes.bin", &store)?;
    }

    let mmapped = MemMappedStore::from_files("/home/folkertdev/Downloads/inspire/postcodes.bin")?;

    // let punt = bounding_box::Point { x: 6.588051, y: 53.334087, };

    let punt = bounding_box::Point {
        x: 5.11007074917847,
        y: 52.062321384871,
    };
    let bb = BoundingBox::around_point(punt, 50.0);

    dbg!(&bb);
    let target = mmapped.postcodes_that_intersect_with(bb);

    let pretty: Vec<_> = target.iter().map(|x| x.to_string()).collect();

    dbg!(pretty);

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
