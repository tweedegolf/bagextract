#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

/// An "inverted" bounding box. The minimal point is infinity, the maximal one negative infinity.
/// This box is an excellent default value: whenever we extend it with an actual point, the
/// bounding box will be just that point. In other words, this property holds:
///
/// INFINITE.extend_with(p) == p
pub const INFINITE: BoundingBox = BoundingBox {
    x_min: f32::INFINITY,
    y_min: f32::INFINITY,
    x_max: f32::NEG_INFINITY,
    y_max: f32::NEG_INFINITY,
};

impl BoundingBox {
    pub const fn from_point(point: Point) -> Self {
        Self {
            x_min: point.x,
            y_min: point.y,
            x_max: point.x,
            y_max: point.y,
        }
    }

    pub fn around_point(point: Point, distance: f32) -> Self {
        let long = point.x;
        let lat = point.y;

        // rather imprecise...

        let factor = 10001.965729; // kilometers per 90 degrees

        let per_m = 90.0 / (factor * 1000.0);

        let frac = ((distance as f64) * per_m) as f32;

        Self {
            x_min: long - frac,
            y_min: lat - frac,
            x_max: long + frac,
            y_max: lat + frac,
        }
    }

    #[inline]
    pub fn extend_with(&mut self, point: Point) {
        self.x_min = self.x_min.min(point.x);
        self.y_min = self.y_min.min(point.y);
        self.x_max = self.x_max.max(point.x);
        self.y_max = self.y_max.max(point.y);
    }

    #[inline]
    pub fn intersects_with(&self, other: Self) -> bool {
        let bounding_box = self;

        (bounding_box.x_min <= (other.x_max))
            && (bounding_box.x_max >= (other.x_min))
            && (bounding_box.y_min <= (other.y_max))
            && (bounding_box.y_max >= (other.y_min))
    }

    #[inline]
    pub fn is_infinite(&self) -> bool {
        self.x_min == INFINITE.x_min
            || self.y_min == INFINITE.y_min
            || self.x_max == INFINITE.x_max
            || self.y_max == INFINITE.y_max
    }

    pub fn corners(&self) -> [Point; 4] {
        [
            Point::new(self.x_min, self.y_min),
            Point::new(self.x_min, self.y_max),
            Point::new(self.x_max, self.y_max),
            Point::new(self.x_max, self.y_min),
        ]
    }

    /// Writes to the postgis format, e.g.
    ///
    /// POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))
    pub fn write_postgis(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(w, "POLYGON((")?;

        let corners = self.corners();

        let mut it = corners.iter().peekable();
        while let Some(corner) = it.next() {
            write!(w, "{} {}", corner.x, corner.y)?;

            if it.peek().is_some() {
                write!(w, ",")?;
            }
        }

        write!(w, "))")
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn from_rijksdriehoek(x: f32, y: f32) -> Self {
        let (y, x) = rijksdriehoek::rijksdriehoek_to_wgs84(x, y);

        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f32 {
        let a = (self.x - other.x).powi(2);
        let b = (self.y - other.y).powi(2);

        (a + b).sqrt()
    }

    pub fn write_postgis(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(w, "POINT({} {})", self.x, self.y)
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
