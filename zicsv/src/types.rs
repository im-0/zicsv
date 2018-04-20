use std;

use chrono;
use ipnet;

use url;
#[cfg(feature = "serialization")]
use url_serde;

#[cfg(feature = "serialization")]
use ipnet_serde;

/// Internet address blocked by Roskomnadzor.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
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
#[derive(Debug)]
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
