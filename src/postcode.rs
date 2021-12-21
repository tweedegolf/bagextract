#[derive(PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct CompactPostcode {
    digits: u16,
    letters: [u8; 2],
}

impl CompactPostcode {
    pub const fn new(digits: u16, letter1: u8, letter2: u8) -> Self {
        Self {
            digits,
            letters: [letter1, letter2],
        }
    }

    pub fn as_u32(self) -> u32 {
        let digits = self.digits as u32;
        let letter1 = (self.letters[0] - b'A') as u32;
        let letter2 = (self.letters[1] - b'A') as u32;

        debug_assert_eq!(digits, digits & ((1 << 14) - 1));
        debug_assert_eq!(letter1, letter1 & 0b11111);
        debug_assert_eq!(letter2, letter2 & 0b11111);

        let result = (digits << 10) | (letter1 << 5) | letter2;

        debug_assert_eq!(self, Self::from_u32(result));

        result
    }

    pub fn from_u32(input: u32) -> Self {
        let digits = (input >> 10) as u16;

        debug_assert_eq!(digits, digits & ((1 << 14) - 1));

        let letter1 = ((input >> 5) & 0b11111) as u8 + b'A';
        let letter2 = (input & 0b11111) as u8 + b'A';

        Self {
            digits,
            letters: [letter1, letter2],
        }
    }
}

impl ToString for CompactPostcode {
    fn to_string(&self) -> String {
        format!(
            "{}{}{}",
            self.digits, self.letters[0] as char, self.letters[1] as char
        )
    }
}

impl std::fmt::Debug for CompactPostcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompactPostcode")
            .field("digits", &self.digits)
            .field("letters", &self.letters)
            .field("pretty", &self.to_string())
            .finish()
    }
}

impl TryFrom<&str> for CompactPostcode {
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

        Ok(CompactPostcode { digits, letters })
    }
}

/// Custom serde deserializer so we don't create an intermediate string
impl<'de> serde::de::Deserialize<'de> for CompactPostcode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt;

        struct FieldVisitor;

        impl<'de> serde::de::Visitor<'de> for FieldVisitor {
            type Value = CompactPostcode;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a postcode")
            }

            fn visit_str<E>(self, value: &str) -> Result<CompactPostcode, E>
            where
                E: serde::de::Error,
            {
                match CompactPostcode::try_from(value) {
                    Ok(postcode) => Ok(postcode),
                    Err(e) => Err(E::custom(format!("failed to decode a postcode: {:?}", e))),
                }
            }
        }

        deserializer.deserialize_str(FieldVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn to_and_fro(input: &str) -> String {
        let code = CompactPostcode::try_from(input).unwrap();

        let index = code.as_u32();
        let and_back = CompactPostcode::from_u32(index);

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
