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

#[cfg(test)]
mod tests {
    use std;

    use ipnet;
    use serde_json;

    #[test]
    fn ipnet_serialize() {
        use std::str::FromStr;

        let ipv4_net = ipnet::Ipv4Net::from_str("1.2.3.0/24").unwrap();
        let mut serializer = serde_json::Serializer::new(std::io::Cursor::new(Vec::new()));
        super::serialize(&ipv4_net, &mut serializer).unwrap();
        assert_eq!(
            &serializer.into_inner().into_inner(),
            b"\"1.2.3.0/24\""
        );
    }

    #[test]
    fn ipnet_deserialize() {
        use std::str::FromStr;

        let mut deserializer = serde_json::Deserializer::from_reader(std::io::Cursor::new(b"\"1.2.3.0/24\""));
        let ipv4_net = super::deserialize(&mut deserializer).unwrap();
        assert_eq!(
            ipv4_net,
            ipnet::Ipv4Net::from_str("1.2.3.0/24").unwrap()
        );
    }
}
