#[cfg(feature = "serde_json5")]
type Error = serde_json5::Error;
#[cfg(not(feature = "serde_json5"))]
pub(crate) type Error = serde_json::Error;

pub(crate) fn parse_json<R: std::io::Read, T>(reader: &mut R) -> crate::Result<T> 
where
    for <'de> T: serde::de::Deserialize<'de>,
{
    #[cfg(not(feature = "serde_json5"))]
    use serde_json::from_reader;
    #[cfg(feature = "serde_json5")]
    use serde_json5::from_reader;

    Ok(from_reader(reader)?)
}

pub(crate) fn assert_json5_supported() -> crate::Result<()> {
    if cfg!(not(feature = "serde_json5")) {
        Err(crate::Error::NeedsJson5)
    } else {
        Ok(())
    }
}