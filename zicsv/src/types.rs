use std;

use chrono;
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
    /// Blocked by domain name.
    DomainName(String),
    /// Blocked by wildcard domain name.
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

// TODO: Implement TryFrom<String> for Address

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
}
