use ipnet;
use serde;

pub fn serialize<S>(value: &ipnet::Ipv4Net, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;

    format!("{}/{}", value.addr(), value.prefix_len()).serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<ipnet::Ipv4Net, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use std::error::Error;
    use std::str::FromStr;
    use serde::Deserialize;

    let str_subnet = String::deserialize(deserializer)?;
    ipnet::Ipv4Net::from_str(&str_subnet).map_err(|error| serde::de::Error::custom(error.description()))
}
