pub mod bounding_box;
pub mod memory_mapped_slice;
pub mod parse_num;
pub mod parse_vbo;
pub mod postcode;

use std::ops::Range;

pub fn in_steps(full_range: Range<usize>, steps: usize) -> Vec<Range<usize>> {
    let delta = full_range.end - full_range.start;
    let step = (delta as f32 / steps as f32).ceil() as usize;

    let xs = (0..).map(|x| x * step).take_while(|x| *x < full_range.end);

    let ys = (1..)
        .map(|x| x * step)
        .take_while(|x| *x < full_range.end)
        .chain(std::iter::once(full_range.end));

    Vec::from_iter(xs.zip(ys).map(|(l, h)| l..h))
}

#[cfg(test)]
mod range_test {
    #[test]
    fn identity() {
        let actual = super::in_steps(0..100, 1);
        let expected = vec![0..100];

        assert_eq!(expected, actual);
    }

    #[test]
    fn two() {
        let actual = super::in_steps(0..100, 2);
        let expected = vec![0..50, 50..100];

        assert_eq!(expected, actual);
    }

    #[test]
    fn three() {
        let actual = super::in_steps(0..100, 3);
        let expected = vec![0..34, 34..68, 68..100];

        assert_eq!(expected, actual);
    }
}
