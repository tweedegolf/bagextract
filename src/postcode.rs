#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Postcode {
    bytes: [u8; 3],
}

impl PartialOrd for Postcode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_u32().partial_cmp(&other.as_u32())
    }
}

impl Ord for Postcode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_u32().cmp(&other.as_u32())
    }
}

impl Postcode {
    #[inline]
    pub const fn new(digits: u16, letter1: u8, letter2: u8) -> Self {
        let digits = digits as u32;
        let letter1 = (letter1 - b'A') as u32;
        let letter2 = (letter2 - b'A') as u32;

        let result = (digits << 10) | (letter1 << 5) | letter2;

        let mut output = [0; 3];

        let bytes = result.to_le_bytes();

        output[0] = bytes[0];
        output[1] = bytes[1];
        output[2] = bytes[2];

        Self { bytes: output }
    }

    pub const fn from_u32(index: u32) -> Self {
        let le_bytes = index.to_le_bytes();
        let mut bytes = [0; 3];

        bytes[0] = le_bytes[0];
        bytes[1] = le_bytes[1];
        bytes[2] = le_bytes[2];

        Self { bytes }
    }

    pub const fn from_index(index: usize) -> Self {
        Self::from_u32(index as u32)
    }

    pub const fn as_u32(self) -> u32 {
        let mut le_bytes = [0; 4];

        le_bytes[0] = self.bytes[0];
        le_bytes[1] = self.bytes[1];
        le_bytes[2] = self.bytes[2];

        u32::from_le_bytes(le_bytes)
    }

    pub const fn as_index(self) -> usize {
        Self::as_u32(self) as usize
    }

    pub const fn components(self) -> (u16, u8, u8) {
        let input = self.as_u32();
        let digits = (input >> 10) as u16;

        let letter1 = ((input >> 5) & 0b11111) as u8 + b'A';
        let letter2 = (input & 0b11111) as u8 + b'A';

        (digits, letter1, letter2)
    }
}

impl ToString for Postcode {
    fn to_string(&self) -> String {
        let (digits, letter1, letter2) = self.components();
        format!("{}{}{}", digits, letter1 as char, letter2 as char)
    }
}

impl std::fmt::Debug for Postcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompactPostcode")
            .field("pretty", &self.to_string())
            .finish()
    }
}

impl TryFrom<&str> for Postcode {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 6 {
            return Err(());
        }

        let digits: u16 = match &value[..4].parse() {
            Ok(v) => *v,
            Err(e) => {
                panic!("{}", e);
            }
        };

        let letters: [u8; 2] = value[4..6].as_bytes().try_into().unwrap();

        Ok(Postcode::new(digits, letters[0], letters[1]))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn to_and_fro(input: &str) -> String {
        let code = Postcode::try_from(input).unwrap();

        let index = code.as_u32();
        let and_back = Postcode::from_u32(index);

        and_back.to_string()
    }

    #[test]
    fn goenga() {
        let input = "8628ET";

        assert_eq!(input, &to_and_fro(input))
    }

    #[test]
    fn highest() {
        let input = "9999ZZ";

        assert_eq!(input, &to_and_fro(input))
    }
}
