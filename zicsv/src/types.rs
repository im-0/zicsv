use std;

use chrono;
use failure;
use ipnet;

use url;
#[cfg(feature = "serialization")]
use url_serde;

#[cfg(feature = "serialization")]
use ipnet_serde;

/// Internet address blocked by Roskomnadzor.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Address {
    /// Blocked by IPv4 address.
    IPv4(std::net::Ipv4Addr),
    /// Blocked by IPv4 subnet.
    #[cfg_attr(feature = "serialization", serde(with = "ipnet_serde"))]
    IPv4Network(ipnet::Ipv4Net),
    /// Blocked by domain name. Lowercase, in IDN format (punicode encoded if necessary).
    DomainName(String),
    /// Blocked by wildcard domain name. Lowercase, in IDN format (punicode encoded if necessary).
    WildcardDomainName(String),
    /// Blocked by URL.
    #[cfg_attr(feature = "serialization", serde(with = "url_serde"))]
    URL(url::Url),

    #[doc(hidden)]
    /// This enum may be extended in future, use catch-all `_` arm to match future variants.
    __NonExhaustive,
}

pub type Addresses = Vec<Address>;
pub type Date = chrono::NaiveDate;

/// One record from CSV.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Record {
    /// Blocked addresses.
    pub addresses: Addresses,
    /// Name of organization that requested blocking.
    pub organization: String,
    /// ID of official document.
    pub document_id: String,
    /// Date of official document.
    pub document_date: Date,

    #[doc(hidden)]
    /// This struct may be extended in future.
    #[cfg_attr(feature = "serialization", serde(skip_serializing))]
    pub(crate) __may_be_extended: (),
}

pub type DateTime = chrono::NaiveDateTime;

impl Address {
    fn add_context<T, E>(orig_address: &str, address: Result<T, E>) -> Result<T, failure::Error>
    where
        E: failure::Fail,
    {
        address.map_err(|error| error.context(format!("Address: \"{}\"", orig_address)).into())
    }

    fn add_context_failure<T>(orig_address: &str, address: Result<T, failure::Error>) -> Result<T, failure::Error> {
        address.map_err(|error| error.context(format!("Address: \"{}\"", orig_address)).into())
    }

    pub fn ipv4_from_str(address: &str) -> Result<Self, failure::Error> {
        Ok(Address::IPv4(Self::add_context(address, address.parse())?))
    }

    pub fn ipv4_network_from_str(address: &str) -> Result<Self, failure::Error> {
        Ok(Address::IPv4Network(Self::add_context(address, address.parse())?))
    }

    fn str_to_idn_punycode(address: &str) -> Result<String, failure::Error> {
        url::idna::domain_to_ascii(address).map_err(|_| format_err!("Unable to convert domain name to publycode"))
    }

    fn domain_name_from_str_no_ctx(address: &str) -> Result<Self, failure::Error> {
        ensure!(!address.is_empty(), "Empty domain name");
        Ok(Address::DomainName(Self::str_to_idn_punycode(address)?))
    }

    pub fn domain_name_from_str(address: &str) -> Result<Self, failure::Error> {
        Self::add_context_failure(address, Self::domain_name_from_str_no_ctx(address))
    }

    fn wildcard_domain_name_from_str_no_ctx(address: &str) -> Result<Self, failure::Error> {
        ensure!(
            address.starts_with("*.") || address == "*",
            "Invalid wildcard domain name"
        );
        Ok(Address::WildcardDomainName(Self::str_to_idn_punycode(address)?))
    }

    pub fn wildcard_domain_name_from_str(address: &str) -> Result<Self, failure::Error> {
        Self::add_context_failure(address, Self::wildcard_domain_name_from_str_no_ctx(address))
    }

    pub fn url_from_str(address: &str) -> Result<Self, failure::Error> {
        Ok(Address::URL(Self::add_context(address, address.parse())?))
    }
}

impl<'a> From<&'a Address> for String {
    fn from(address: &Address) -> Self {
        #[allow(non_snake_case)]
        match address {
            &Address::IPv4(value) => format!("{}", value),
            &Address::IPv4Network(value) => format!("{}/{}", value.addr(), value.prefix_len()),

            &Address::DomainName(ref value) | &Address::WildcardDomainName(ref value) => value.clone(),

            &Address::URL(ref value) => value.as_str().into(),

            __NonExhaustive => unreachable!(),
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "{}", String::from(self))
    }
}

impl std::str::FromStr for Address {
    type Err = failure::Error;

    fn from_str(address: &str) -> Result<Self, Self::Err> {
        Self::ipv4_from_str(address)
            .or_else(|_| Self::ipv4_network_from_str(address))
            .or_else(|_| Self::url_from_str(address))
            .or_else(|_| Self::wildcard_domain_name_from_str(address))
            .or_else(|_| Self::domain_name_from_str(address))
            .map_err(|_| format_err!("Unknown type of address: \"{}\"", address))
    }
}

impl std::fmt::Display for Record {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        {
            let mut addresses_iter = self.addresses.iter();
            if let Some(first_address) = addresses_iter.next() {
                write!(formatter, "{}", first_address)?;
                for address in addresses_iter {
                    write!(formatter, ", {}", address)?;
                }
                write!(formatter, " ")?;
            }
            write!(
                formatter,
                "(\"{}\", \"{}\"/{})",
                self.organization, self.document_id, self.document_date
            )
        }
    }
}

impl std::default::Default for Record {
    fn default() -> Self {
        Self {
            addresses: Vec::default(),
            organization: String::default(),
            document_id: String::default(),
            document_date: chrono::NaiveDate::from_ymd(1970, 1, 1),

            __may_be_extended: (),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono;

    #[test]
    fn string_from_address() {
        assert_eq!(
            String::from(&super::Address::IPv4("1.2.3.4".parse().unwrap())),
            "1.2.3.4"
        );

        assert_eq!(
            String::from(&super::Address::IPv4Network("1.2.3.0/24".parse().unwrap())),
            "1.2.3.0/24"
        );

        assert_eq!(
            String::from(&super::Address::DomainName("example.com".into())),
            "example.com"
        );

        assert_eq!(
            String::from(&super::Address::WildcardDomainName("*.example.com".into())),
            "*.example.com"
        );

        assert_eq!(
            String::from(&super::Address::URL("http://example.com/".parse().unwrap())),
            "http://example.com/"
        );
    }

    #[test]
    fn display_record() {
        let record = super::Record {
            addresses: vec![],
            organization: "Test org".into(),
            document_id: "Test document ID".into(),
            document_date: chrono::NaiveDate::from_ymd(2017, 1, 2),

            ..super::Record::default()
        };
        assert_eq!(format!("{}", record), "(\"Test org\", \"Test document ID\"/2017-01-02)");

        let record = super::Record {
            addresses: vec![super::Address::DomainName("example.com".into())],
            organization: "Test org".into(),
            document_id: "Test document ID".into(),
            document_date: chrono::NaiveDate::from_ymd(2017, 1, 2),

            ..super::Record::default()
        };
        assert_eq!(
            format!("{}", record),
            "example.com (\"Test org\", \"Test document ID\"/2017-01-02)"
        );

        let record = super::Record {
            addresses: vec![
                super::Address::DomainName("example.com".into()),
                super::Address::IPv4("1.2.3.4".parse().unwrap()),
            ],
            organization: "Test org".into(),
            document_id: "Test document ID".into(),
            document_date: chrono::NaiveDate::from_ymd(2017, 1, 2),

            ..super::Record::default()
        };
        assert_eq!(
            format!("{}", record),
            "example.com, 1.2.3.4 (\"Test org\", \"Test document ID\"/2017-01-02)"
        );
    }

    #[test]
    fn address_from_str() {
        assert!("".parse::<super::Address>().is_err());

        assert_eq!(
            "127.0.0.1".parse::<super::Address>().unwrap(),
            super::Address::IPv4("127.0.0.1".parse().unwrap())
        );

        assert_eq!(
            "127.0.0.0/8".parse::<super::Address>().unwrap(),
            super::Address::IPv4Network("127.0.0.0/8".parse().unwrap())
        );

        assert_eq!(
            "http://example.com".parse::<super::Address>().unwrap(),
            super::Address::URL("http://example.com".parse().unwrap())
        );
        assert_eq!(
            "http://EXAMPLE.com".parse::<super::Address>().unwrap(),
            super::Address::URL("http://example.com".parse().unwrap())
        );
        assert_eq!(
            "http://\u{442}\u{435}\u{441}\u{442}.org/test"
                .parse::<super::Address>()
                .unwrap(),
            super::Address::URL("http://xn--e1aybc.org/test".parse().unwrap())
        );
        // Uppercase.
        assert_eq!(
            "http://\u{422}\u{415}\u{421}\u{422}.org/test"
                .parse::<super::Address>()
                .unwrap(),
            super::Address::URL("http://xn--e1aybc.org/test".parse().unwrap())
        );

        assert_eq!(
            "*".parse::<super::Address>().unwrap(),
            super::Address::WildcardDomainName("*".into())
        );

        assert_eq!(
            "*.example.org".parse::<super::Address>().unwrap(),
            super::Address::WildcardDomainName("*.example.org".into())
        );
        assert_eq!(
            "*.EXAMPLE.org".parse::<super::Address>().unwrap(),
            super::Address::WildcardDomainName("*.example.org".into())
        );
        assert_eq!(
            "*.\u{442}\u{435}\u{441}\u{442}.org".parse::<super::Address>().unwrap(),
            super::Address::WildcardDomainName("*.xn--e1aybc.org".into())
        );
        // Uppercase.
        assert_eq!(
            "*.\u{422}\u{415}\u{421}\u{422}.org".parse::<super::Address>().unwrap(),
            super::Address::WildcardDomainName("*.xn--e1aybc.org".into())
        );
        assert_eq!(
            "*.xn--e1aybc.org".parse::<super::Address>().unwrap(),
            super::Address::WildcardDomainName("*.xn--e1aybc.org".into())
        );
        assert_eq!(
            "*.XN--E1AYBC.org".parse::<super::Address>().unwrap(),
            super::Address::WildcardDomainName("*.xn--e1aybc.org".into())
        );

        assert_eq!(
            "example.org".parse::<super::Address>().unwrap(),
            super::Address::DomainName("example.org".into())
        );
        assert_eq!(
            "EXAMPLE.org".parse::<super::Address>().unwrap(),
            super::Address::DomainName("example.org".into())
        );
        assert_eq!(
            "\u{442}\u{435}\u{441}\u{442}.org".parse::<super::Address>().unwrap(),
            super::Address::DomainName("xn--e1aybc.org".into())
        );
        // Uppercase.
        assert_eq!(
            "\u{422}\u{415}\u{421}\u{422}.org".parse::<super::Address>().unwrap(),
            super::Address::DomainName("xn--e1aybc.org".into())
        );
        assert_eq!(
            "xn--e1aybc.org".parse::<super::Address>().unwrap(),
            super::Address::DomainName("xn--e1aybc.org".into())
        );
        assert_eq!(
            "XN--E1AYBC.ORG".parse::<super::Address>().unwrap(),
            super::Address::DomainName("xn--e1aybc.org".into())
        );
    }
}
