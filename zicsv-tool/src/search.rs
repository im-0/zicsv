use std;

use failure;
use serde;
use serde_json;
use url;

use zicsv;

use print_err;

fn extract_more_info(address: &zicsv::Address) -> Vec<Result<zicsv::Address, failure::Error>> {
    match *address {
        zicsv::Address::IPv4(_) | zicsv::Address::IPv4Network(_) | zicsv::Address::WildcardDomainName(_) => vec![],

        zicsv::Address::URL(ref url) => match url.host() {
            Some(host) => match host {
                url::Host::Domain(domain) => vec![Ok(zicsv::Address::DomainName(domain.into()))],
                url::Host::Ipv4(ipv4_addr) => vec![Ok(zicsv::Address::IPv4(ipv4_addr))],
                url::Host::Ipv6(ipv6_addr) => vec![Err(format_err!("URL contains IPv6 address: {}", ipv6_addr))],
            },

            None => vec![],
        },

        // TODO: Resolve IP addresses.
        zicsv::Address::DomainName(_) => vec![],

        _ => vec![],
    }
}

fn extract_all_info(address: &str, n_errors: &mut usize) -> Result<zicsv::Addresses, failure::Error> {
    let mut extracted = zicsv::Addresses::new();
    extracted.push(address
        .parse()
        .map_err(|error: failure::Error| error.context(format!("Address: \"{}\"", address)))?);

    let mut next_n = 0;
    while next_n < extracted.len() {
        let more_info = extract_more_info(&extracted[next_n]);
        extracted.extend(
            more_info
                .into_iter()
                .map(|item| {
                    item.map_err(|error| {
                        *n_errors += 1;
                        print_err::print_error(&error.context(format!("Original address: \"{}\"", address)).into());
                    })
                })
                .filter_map(Result::ok),
        );
        next_n += 1;
    }

    Ok(extracted)
}

mod serialize_rc_record {
    use super::*;

    pub fn serialize<S>(value: &std::rc::Rc<zicsv::Record>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::Serialize;

        value.serialize(serializer)
    }
}

#[derive(Debug, Serialize)]
enum MatchReason {
    /// IPv4 address is equal to blocked IPv4 address.
    IPv4Equals,
    /// IPv4 address is contained in blocked IPv4 network.
    IPv4InBlockedIPv4Network,

    /// IPv4 network contains blocked IPv4 address.
    IPv4NetworkContainsBlockedIPv4,
    /// IPv4 network is equal to blocked IPv4 network.
    IPv4NetworkEquals,
    /// IPv4 network is a subset of blocked IPv4 network.
    IPv4NetworkInBlockedIPv4Network,
    /// IPV4 network is a superset of blocked IPv4 network.
    IPv4NetworkContainsBlockedIPv4Network,

    /// Domain name is equal to blocked domain name.
    DomainNameEquals,
    /// Domain name matches blocked wildcard domain name.
    DomainNameInBlockedWildcard,
    /// Domain name is used in blocked URL.
    DomainNameInBlockedURL,

    /// Wildcard domain name is matched by blocked domain name.
    WildcardContainsBlockedDomain,
    /// Wildcard domain name is equal to blocked wildcard domain name.
    WildcardEquals,
    /// Wildcard domain name is a subset of blocked wildcard domain name.
    WildcardInBlockedWildcard,
    /// Wildcard domain name is a superset of blocked wildcard domain name.
    WildcardContainsBlockedWildcard,
    /// Wildcard domain name is matched by host name of blocked URL.
    WildcardContainsBlockedURL,

    /// URL is equal to blocked URL.
    URLEquals,
    /// URL is a base of blocked URL.
    URLContainsBlockedURL,
    /// URL is starting from blocked URL.
    URLInBlockedURL,
}

#[derive(Debug, Serialize)]
struct Match {
    #[serde(with = "serialize_rc_record")]
    block_record: std::rc::Rc<zicsv::Record>,

    blocked_address: zicsv::Address,
    match_reason: MatchReason,
}

#[derive(Debug, Serialize)]
struct SubAddressWithMatches {
    address: zicsv::Address,
    matches: Vec<Match>,
}

#[derive(Debug, Serialize)]
struct Address<'a> {
    original_address: &'a str,
    addresses: Vec<SubAddressWithMatches>,
}

fn match_wildcard_domain(wildcard_domain: &str, domain: &str) -> bool {
    (wildcard_domain == "*") || domain.ends_with(wildcard_domain.trim_left_matches('*'))
}

fn addr_match(blocked_address: &zicsv::Address, address: &zicsv::Address) -> Option<MatchReason> {
    use ipnet::Contains;

    match *address {
        zicsv::Address::IPv4(ipv4) => match *blocked_address {
            zicsv::Address::IPv4(blocked_ipv4) => if blocked_ipv4 == ipv4 {
                Some(MatchReason::IPv4Equals)
            } else {
                None
            },

            zicsv::Address::IPv4Network(blocked_ipv4_net) => if blocked_ipv4_net.contains(&ipv4) {
                Some(MatchReason::IPv4InBlockedIPv4Network)
            } else {
                None
            },

            _ => None,
        },

        zicsv::Address::IPv4Network(ipv4_net) => match *blocked_address {
            zicsv::Address::IPv4(blocked_ipv4) => if ipv4_net.contains(&blocked_ipv4) {
                Some(MatchReason::IPv4NetworkContainsBlockedIPv4)
            } else {
                None
            },

            zicsv::Address::IPv4Network(blocked_ipv4_net) => if blocked_ipv4_net == ipv4_net {
                Some(MatchReason::IPv4NetworkEquals)
            } else if blocked_ipv4_net.contains(&ipv4_net) {
                Some(MatchReason::IPv4NetworkInBlockedIPv4Network)
            } else if ipv4_net.contains(&blocked_ipv4_net) {
                Some(MatchReason::IPv4NetworkContainsBlockedIPv4Network)
            } else {
                None
            },

            _ => None,
        },

        zicsv::Address::DomainName(ref domain) => match *blocked_address {
            zicsv::Address::DomainName(ref blocked_domain) => if blocked_domain == domain {
                Some(MatchReason::DomainNameEquals)
            } else {
                None
            },

            zicsv::Address::WildcardDomainName(ref blocked_wildcard) => {
                if match_wildcard_domain(blocked_wildcard, domain) {
                    Some(MatchReason::DomainNameInBlockedWildcard)
                } else {
                    None
                }
            },

            zicsv::Address::URL(ref blocked_url) => blocked_url.host_str().and_then(|blocked_domain| {
                if blocked_domain == domain {
                    Some(MatchReason::DomainNameInBlockedURL)
                } else {
                    None
                }
            }),

            _ => None,
        },

        zicsv::Address::WildcardDomainName(ref wildcard) => match *blocked_address {
            zicsv::Address::DomainName(ref blocked_domain) => if match_wildcard_domain(wildcard, blocked_domain) {
                Some(MatchReason::WildcardContainsBlockedDomain)
            } else {
                None
            },

            zicsv::Address::WildcardDomainName(ref blocked_wildcard) => if blocked_wildcard == wildcard {
                Some(MatchReason::WildcardEquals)
            } else if match_wildcard_domain(blocked_wildcard, wildcard) {
                Some(MatchReason::WildcardInBlockedWildcard)
            } else if match_wildcard_domain(wildcard, blocked_wildcard) {
                Some(MatchReason::WildcardContainsBlockedWildcard)
            } else {
                None
            },

            zicsv::Address::URL(ref blocked_url) => blocked_url.host_str().and_then(|blocked_domain| {
                if match_wildcard_domain(wildcard, blocked_domain) {
                    Some(MatchReason::WildcardContainsBlockedURL)
                } else {
                    None
                }
            }),

            _ => None,
        },

        zicsv::Address::URL(ref url) => match *blocked_address {
            zicsv::Address::URL(ref blocked_url) => if blocked_url == url {
                Some(MatchReason::URLEquals)
            } else if blocked_url.as_str().starts_with(url.as_str()) {
                Some(MatchReason::URLContainsBlockedURL)
            } else if url.as_str().starts_with(blocked_url.as_str()) {
                Some(MatchReason::URLInBlockedURL)
            } else {
                None
            },

            // extract_more_info() already extracts domain names from URLs, so there is no need to match DomainName
            // and WildcardDomainName here.
            _ => None,
        },

        _ => None,
    }
}

fn find_matches(block_record: &std::rc::Rc<zicsv::Record>, address: &mut Address) {
    for blocked_address in &block_record.addresses {
        for sub_address in &mut address.addresses {
            if let Some(match_reason) = addr_match(&blocked_address, &sub_address.address) {
                sub_address.matches.push(Match {
                    block_record: block_record.clone(),
                    blocked_address: blocked_address.clone(),
                    match_reason,
                })
            }
        }
    }
}

pub fn search<StreamWriter>(
    orig_addresses: &[String],
    mut reader: Box<zicsv::GenericReader>,
    writer: &mut StreamWriter,
    output_format: &super::OutputFormat,
) -> Result<(), failure::Error>
where
    StreamWriter: std::io::Write,
{
    let mut n_prepare_errors = 0usize;
    let mut n_reader_errors = 0usize;

    let addresses: Result<Vec<_>, _> = orig_addresses
        .into_iter()
        .map(|orig_address| {
            extract_all_info(orig_address.trim(), &mut n_prepare_errors).map(|sub_addresses| Address {
                original_address: orig_address,
                addresses: sub_addresses
                    .into_iter()
                    .map(|sub_address| SubAddressWithMatches {
                        address: sub_address,
                        matches: Vec::new(),
                    })
                    .collect(),
            })
        })
        .collect();
    let mut addresses = addresses?;

    for record in reader.iter() {
        match record {
            Ok(record) => {
                let record = std::rc::Rc::new(record);
                for address in &mut addresses {
                    find_matches(&record, address);
                }
            },

            Err(error) => {
                n_reader_errors += 1;
                print_err::print_error(&error);
            },
        }
    }

    match *output_format {
        super::OutputFormat::PrettyJSON => serde_json::to_writer_pretty(writer, &addresses)?,
        super::OutputFormat::JSON => serde_json::to_writer(writer, &addresses)?,
        // TODO: Human-readable output.
    }

    ensure!(
        n_prepare_errors == 0,
        "{} errors occur while extracting addresses",
        n_prepare_errors
    );
    ensure!(
        n_reader_errors == 0,
        "{} errors occur while reading list",
        n_reader_errors
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use zicsv;

    #[test]
    fn extract_more_info() {
        assert_eq!(
            super::extract_more_info(&zicsv::Address::IPv4("127.0.0.1".parse().unwrap()))
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![]
        );

        assert_eq!(
            super::extract_more_info(&zicsv::Address::IPv4Network("127.0.0.0/8".parse().unwrap()))
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![]
        );

        assert_eq!(
            super::extract_more_info(&zicsv::Address::URL("http://example.org/".parse().unwrap()))
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![Some(zicsv::Address::DomainName("example.org".into()))]
        );
        assert_eq!(
            super::extract_more_info(&zicsv::Address::URL("http://1.2.3.4/".parse().unwrap()))
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![Some(zicsv::Address::IPv4("1.2.3.4".parse().unwrap()))]
        );
        // IPv6 addresses are not currently supported.
        assert_eq!(
            super::extract_more_info(&zicsv::Address::URL(
                "http://[1080::8:800:200C:417A]/foo".parse().unwrap()
            )).into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![None]
        );

        assert_eq!(
            super::extract_more_info(&zicsv::Address::WildcardDomainName("*.example.org".into()))
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![]
        );

        // TODO: How to test this?
        assert_eq!(
            super::extract_more_info(&zicsv::Address::DomainName("example.org".into()))
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![]
        );
    }

    #[test]
    fn extract_all_info() {
        let mut n_errors = 0usize;
        assert!(super::extract_all_info("", &mut n_errors).is_err());
        assert_eq!(n_errors, 0);

        let mut n_errors = 0usize;
        assert_eq!(
            super::extract_all_info("127.0.0.1", &mut n_errors).unwrap(),
            vec![zicsv::Address::IPv4("127.0.0.1".parse().unwrap())]
        );
        assert_eq!(n_errors, 0);

        // TODO: How to test name resolution?
        let mut n_errors = 0usize;
        assert_eq!(
            super::extract_all_info("http://example.org", &mut n_errors).unwrap(),
            vec![
                zicsv::Address::URL("http://example.org".parse().unwrap()),
                zicsv::Address::DomainName("example.org".into()),
            ]
        );
        assert_eq!(n_errors, 0);

        let mut n_errors = 0usize;
        assert_eq!(
            super::extract_all_info("http://1.2.3.4", &mut n_errors).unwrap(),
            vec![
                zicsv::Address::URL("http://1.2.3.4".parse().unwrap()),
                zicsv::Address::IPv4("1.2.3.4".parse().unwrap()),
            ]
        );
        assert_eq!(n_errors, 0);

        let mut n_errors = 0usize;
        assert_eq!(
            super::extract_all_info("http://[1080::8:800:200C:417A]", &mut n_errors).unwrap(),
            vec![zicsv::Address::URL("http://[1080::8:800:200C:417A]".parse().unwrap())]
        );
        assert_eq!(n_errors, 1);
    }

    // TODO: Test addr_match().
}
