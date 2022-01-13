#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f32 {
        let a = (self.x - other.x).powi(2);
        let b = (self.y - other.y).powi(2);

        (a + b).sqrt()
    }
}

impl std::str::FromStr for Point {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .collect();

        let x_fromstr = coords[0].parse::<f32>()?;
        let y_fromstr = coords[1].parse::<f32>()?;

        Ok(Point {
            x: x_fromstr,
            y: y_fromstr,
        })
    }
}
