// parse the wrapper around the data in the zip file
use serde::de::Deserialize;
use serde_derive::Deserialize;

#[derive(Debug)]
pub struct Wrapper<T> {
    pub objects: Vec<T>,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Wrapper<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let help: WrapperHelp<T> = Deserialize::deserialize(deserializer)?;

        let result = Wrapper {
            objects: help.antwoord.producten.product.objects,
        };

        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename = "xb:BAG-Extract-Deelbestand-LVC")]
pub struct WrapperHelp<T> {
    antwoord: Antwoord<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "xb:antwoord")]
struct Antwoord<T> {
    producten: Producten<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "xb:producten")]
struct Producten<T> {
    #[serde(alias = "LVC-product")]
    product: Product<T>,
}

#[derive(Debug, Deserialize)]
struct Product<T> {
    #[serde(rename = "$value")]
    objects: Vec<T>,
}
