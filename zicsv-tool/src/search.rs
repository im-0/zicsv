use std;

use failure;
use serde;
use serde_json;
use trust_dns_proto;
use trust_dns_resolver;
use url;

use zicsv;

use print_err;

fn resolve_helper<T, F>(
    record_type: &str,
    resolve_result: trust_dns_resolver::error::ResolveResult<T>,
    f: F,
) -> Vec<Result<zicsv::Address, failure::Error>>
where
    F: Fn(T) -> Vec<Result<zicsv::Address, failure::Error>>,
{
    match resolve_result {
        Ok(response) => f(response),

        Err(error) => match *error.kind() {
            trust_dns_resolver::error::ResolveErrorKind::NoRecordsFound(_) => vec![],
            _ => vec![Err(format_err!("{} resolution: {}", record_type, error))],
        },
    }
}

fn extract_more_info(
    address: &zicsv::Address,
    resolver: &trust_dns_resolver::Resolver,
) -> Vec<Result<zicsv::Address, failure::Error>> {
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

        zicsv::Address::DomainName(ref domain) => {
            // TODO: Resolve other types of records containing IP addresses and host aliases.
            // TODO: Try to resolve multiple times.
            // For some hosts (example: google.com) DNS server may return different addresses every time.
            let mut all_resolved = resolve_helper("IP", resolver.lookup_ip(domain), |response| {
                response
                    .iter()
                    .filter_map(|resolved_addr| match resolved_addr {
                        std::net::IpAddr::V4(ipv4_addr) => Some(Ok(zicsv::Address::IPv4(ipv4_addr))),
                        // IPv6 is not supported.
                        _ => None,
                    })
                    .collect()
            });

            let mut cname_resolved = resolve_helper(
                "CNAME",
                resolver.lookup(domain, trust_dns_proto::rr::record_type::RecordType::CNAME),
                |response| {
                    response
                        .iter()
                        .filter_map(|resolved_cname| match *resolved_cname {
                            trust_dns_proto::rr::record_data::RData::CNAME(ref cname) => {
                                Some(zicsv::Address::domain_name_from_str(&cname.to_utf8()))
                            },

                            // Ignore other types.
                            _ => None,
                        })
                        .collect()
                },
            );
            all_resolved.extend(cname_resolved.drain(..));

            all_resolved
        },

        _ => vec![],
    }
}

fn extract_all_info(
    address: &str,
    resolver: &trust_dns_resolver::Resolver,
    n_errors: &mut usize,
) -> Result<zicsv::Addresses, failure::Error> {
    let mut extracted = zicsv::Addresses::new();
    extracted.push(address
        .parse()
        .map_err(|error: failure::Error| error.context(format!("Address: \"{}\"", address)))?);

    let mut next_n = 0;
    while next_n < extracted.len() {
        let more_info = extract_more_info(&extracted[next_n], resolver);
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

#[derive(Debug, PartialEq, Serialize)]
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

    /// Wildcard domain name is matched by blocked domain name.
    WildcardContainsBlockedDomain,
    /// Wildcard domain name is equal to blocked wildcard domain name.
    WildcardEquals,
    /// Wildcard domain name is a subset of blocked wildcard domain name.
    WildcardInBlockedWildcard,
    /// Wildcard domain name is a superset of blocked wildcard domain name.
    WildcardContainsBlockedWildcard,

    /// URL is equal to blocked URL.
    URLEquals,
    /// URL is a base of blocked URL.
    URLContainsBlockedURL,
    /// URL is starting from blocked URL.
    URLInBlockedURL,
}

impl std::fmt::Display for MatchReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            formatter,
            "{}",
            match *self {
                MatchReason::IPv4Equals => "IPv4 address is equal to blocked IPv4 address",
                MatchReason::IPv4InBlockedIPv4Network => "IPv4 address is contained in blocked IPv4 network",

                MatchReason::IPv4NetworkContainsBlockedIPv4 => "IPv4 network contains blocked IPv4 address",
                MatchReason::IPv4NetworkEquals => "IPv4 network is equal to blocked IPv4 network",
                MatchReason::IPv4NetworkInBlockedIPv4Network => "IPv4 network is a subset of blocked IPv4 network",
                MatchReason::IPv4NetworkContainsBlockedIPv4Network => {
                    "IPV4 network is a superset of blocked IPv4 network"
                },

                MatchReason::DomainNameEquals => "Domain name is equal to blocked domain name",
                MatchReason::DomainNameInBlockedWildcard => "Domain name matches blocked wildcard domain name",

                MatchReason::WildcardContainsBlockedDomain => "Wildcard domain name is matched by blocked domain name",
                MatchReason::WildcardEquals => "Wildcard domain name is equal to blocked wildcard domain name",
                MatchReason::WildcardInBlockedWildcard => {
                    "Wildcard domain name is a subset of blocked wildcard domain name"
                },
                MatchReason::WildcardContainsBlockedWildcard => {
                    "Wildcard domain name is a superset of blocked wildcard domain name"
                },

                MatchReason::URLEquals => "URL is equal to blocked URL",
                MatchReason::URLContainsBlockedURL => "URL is a base of blocked URL",
                MatchReason::URLInBlockedURL => "URL is starting from blocked URL",
            }
        )
    }
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

fn create_resolver() -> Result<trust_dns_resolver::Resolver, failure::Error> {
    let (conf, mut opts) = trust_dns_resolver::system_conf::read_system_conf()?;

    // IPv6 is not supported.
    opts.ip_strategy = trust_dns_resolver::config::LookupIpStrategy::Ipv4Only;

    Ok(trust_dns_resolver::Resolver::new(conf, opts)?)
}

fn print_human_readable<StreamWriter>(writer: &mut StreamWriter, addresses: &[Address]) -> Result<(), failure::Error>
where
    StreamWriter: std::io::Write,
{
    let mut not_first = false;
    for address in addresses.iter() {
        if not_first {
            writeln!(writer)?;
        } else {
            not_first = true;
        }

        writeln!(writer, "{}:", address.original_address)?;

        let mut not_first_sub_addr = false;
        for sub_address in &address.addresses {
            if not_first_sub_addr {
                writeln!(writer)?;
            } else {
                not_first_sub_addr = true;
            }

            if sub_address.matches.is_empty() {
                writeln!(writer, "    {}: not found", sub_address.address)?;
            } else {
                writeln!(writer, "    {}: blocked", sub_address.address)?;

                for addr_match in &sub_address.matches {
                    writeln!(writer, "        {}:", addr_match.match_reason)?;
                    writeln!(writer, "            Blocked: {}", addr_match.blocked_address)?;
                    writeln!(
                        writer,
                        "            Organization: {}",
                        addr_match.block_record.organization
                    )?;
                    writeln!(
                        writer,
                        "            Document ID: {}",
                        addr_match.block_record.document_id
                    )?;
                    writeln!(
                        writer,
                        "            Document date: {}",
                        addr_match.block_record.document_date
                    )?;
                }
            }
        }
    }

    Ok(())
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

    let resolver = create_resolver()?;

    let addresses: Result<Vec<_>, _> = orig_addresses
        .into_iter()
        .map(|orig_address| {
            extract_all_info(orig_address.trim(), &resolver, &mut n_prepare_errors).map(|sub_addresses| Address {
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
        super::OutputFormat::HumanReadable => print_human_readable(writer, &addresses)?,
        super::OutputFormat::PrettyJSON => serde_json::to_writer_pretty(writer, &addresses)?,
        super::OutputFormat::JSON => serde_json::to_writer(writer, &addresses)?,
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
    use ipnet;
    use trust_dns_resolver;

    use zicsv;

    fn create_resolver() -> trust_dns_resolver::Resolver {
        // Empty configuration, no DNS servers.
        let conf = trust_dns_resolver::config::ResolverConfig::new();
        // Lookup in /etc/hosts.
        let mut opts = trust_dns_resolver::config::ResolverOpts::default();

        // IPv6 is not supported.
        opts.ip_strategy = trust_dns_resolver::config::LookupIpStrategy::Ipv4Only;

        trust_dns_resolver::Resolver::new(conf, opts).unwrap()
    }

    #[test]
    fn extract_more_info() {
        use ipnet::Contains;

        let resolver = create_resolver();

        assert_eq!(
            super::extract_more_info(&zicsv::Address::IPv4("127.0.0.1".parse().unwrap()), &resolver)
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![]
        );

        assert_eq!(
            super::extract_more_info(&zicsv::Address::IPv4Network("127.0.0.0/8".parse().unwrap()), &resolver)
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![]
        );

        assert_eq!(
            super::extract_more_info(&zicsv::Address::URL("http://example.org/".parse().unwrap()), &resolver)
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![Some(zicsv::Address::DomainName("example.org".into()))]
        );
        assert_eq!(
            super::extract_more_info(&zicsv::Address::URL("http://1.2.3.4/".parse().unwrap()), &resolver)
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![Some(zicsv::Address::IPv4("1.2.3.4".parse().unwrap()))]
        );
        // IPv6 addresses are not currently supported.
        assert_eq!(
            super::extract_more_info(
                &zicsv::Address::URL("http://[1080::8:800:200C:417A]/foo".parse().unwrap()),
                &resolver
            ).into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![None]
        );

        assert_eq!(
            super::extract_more_info(&zicsv::Address::WildcardDomainName("*.example.org".into()), &resolver)
                .into_iter()
                .map(Result::ok)
                .collect::<Vec<Option<zicsv::Address>>>(),
            vec![]
        );

        let mut localhost_addr = super::extract_more_info(&zicsv::Address::DomainName("localhost".into()), &resolver)
            .into_iter()
            .map(Result::ok)
            .collect::<Vec<Option<zicsv::Address>>>();
        assert_eq!(localhost_addr.len(), 1);
        let localhost_addr = localhost_addr.pop().unwrap().unwrap();
        let loopback_net: ipnet::Ipv4Net = "127.0.0.0/8".parse().unwrap();
        match localhost_addr {
            zicsv::Address::IPv4(ipv4_addr) => assert!(loopback_net.contains(&ipv4_addr)),
            invalid_address => panic!("Invalid address: {}", invalid_address),
        }
    }

    #[test]
    fn extract_all_info() {
        use ipnet::Contains;

        let resolver = create_resolver();

        let mut n_errors = 0usize;
        assert!(super::extract_all_info("", &resolver, &mut n_errors).is_err());
        assert_eq!(n_errors, 0);

        let mut n_errors = 0usize;
        assert_eq!(
            super::extract_all_info("127.0.0.1", &resolver, &mut n_errors).unwrap(),
            vec![zicsv::Address::IPv4("127.0.0.1".parse().unwrap())]
        );
        assert_eq!(n_errors, 0);

        let mut n_errors = 0usize;
        let mut from_localhost_url = super::extract_all_info("http://localhost", &resolver, &mut n_errors)
            .unwrap()
            .into_iter();
        assert_eq!(n_errors, 0);
        assert_eq!(
            from_localhost_url.next(),
            Some(zicsv::Address::URL("http://localhost".parse().unwrap()))
        );
        assert_eq!(
            from_localhost_url.next(),
            Some(zicsv::Address::DomainName("localhost".into()))
        );
        let localhost_addr = from_localhost_url.next().unwrap();
        assert_eq!(from_localhost_url.next(), None);
        let loopback_net: ipnet::Ipv4Net = "127.0.0.0/8".parse().unwrap();
        match localhost_addr {
            zicsv::Address::IPv4(ipv4_addr) => assert!(loopback_net.contains(&ipv4_addr)),
            invalid_address => panic!("Invalid address: {}", invalid_address),
        }

        let mut n_errors = 0usize;
        assert_eq!(
            super::extract_all_info("http://1.2.3.4", &resolver, &mut n_errors).unwrap(),
            vec![
                zicsv::Address::URL("http://1.2.3.4".parse().unwrap()),
                zicsv::Address::IPv4("1.2.3.4".parse().unwrap()),
            ]
        );
        assert_eq!(n_errors, 0);

        let mut n_errors = 0usize;
        assert_eq!(
            super::extract_all_info("http://[1080::8:800:200C:417A]", &resolver, &mut n_errors).unwrap(),
            vec![zicsv::Address::URL("http://[1080::8:800:200C:417A]".parse().unwrap())]
        );
        assert_eq!(n_errors, 1);
    }

    #[test]
    fn addr_match() {
        assert_eq!(
            super::addr_match(&"4.3.2.1".parse().unwrap(), &"1.2.3.4".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"1.2.3.4".parse().unwrap(), &"1.2.3.4".parse().unwrap()),
            Some(super::MatchReason::IPv4Equals),
        );
        assert_eq!(
            super::addr_match(&"4.3.0.0/16".parse().unwrap(), &"1.2.3.4".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"1.2.0.0/16".parse().unwrap(), &"1.2.3.4".parse().unwrap()),
            Some(super::MatchReason::IPv4InBlockedIPv4Network),
        );

        assert_eq!(
            super::addr_match(&"4.3.2.1".parse().unwrap(), &"1.2.0.0/16".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"1.2.3.4".parse().unwrap(), &"1.2.0.0/16".parse().unwrap()),
            Some(super::MatchReason::IPv4NetworkContainsBlockedIPv4),
        );
        assert_eq!(
            super::addr_match(&"4.3.0.0/16".parse().unwrap(), &"1.2.0.0/16".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"1.2.0.0/16".parse().unwrap(), &"1.2.0.0/16".parse().unwrap()),
            Some(super::MatchReason::IPv4NetworkEquals),
        );
        assert_eq!(
            super::addr_match(&"4.3.0.0/8".parse().unwrap(), &"1.2.0.0/16".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"1.0.0.0/8".parse().unwrap(), &"1.2.0.0/16".parse().unwrap()),
            Some(super::MatchReason::IPv4NetworkInBlockedIPv4Network),
        );
        assert_eq!(
            super::addr_match(&"4.3.2.0/24".parse().unwrap(), &"1.2.0.0/16".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"1.2.3.0/24".parse().unwrap(), &"1.2.0.0/16".parse().unwrap()),
            Some(super::MatchReason::IPv4NetworkContainsBlockedIPv4Network),
        );

        assert_eq!(
            super::addr_match(&"example.com".parse().unwrap(), &"example.org".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"example.org".parse().unwrap(), &"example.org".parse().unwrap()),
            Some(super::MatchReason::DomainNameEquals),
        );
        assert_eq!(
            super::addr_match(&"*.com".parse().unwrap(), &"example.org".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"*.org".parse().unwrap(), &"example.org".parse().unwrap()),
            Some(super::MatchReason::DomainNameInBlockedWildcard),
        );
        assert_eq!(
            super::addr_match(&"*".parse().unwrap(), &"example.org".parse().unwrap()),
            Some(super::MatchReason::DomainNameInBlockedWildcard),
        );

        assert_eq!(
            super::addr_match(&"test.example.com".parse().unwrap(), &"*.example.org".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"test.example.org".parse().unwrap(), &"*.example.org".parse().unwrap()),
            Some(super::MatchReason::WildcardContainsBlockedDomain),
        );
        assert_eq!(
            super::addr_match(&"*.example.com".parse().unwrap(), &"*.example.org".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"*.example.org".parse().unwrap(), &"*.example.org".parse().unwrap()),
            Some(super::MatchReason::WildcardEquals),
        );
        assert_eq!(
            super::addr_match(&"*.com".parse().unwrap(), &"*.example.org".parse().unwrap()),
            None,
        );
        assert_eq!(
            super::addr_match(&"*.org".parse().unwrap(), &"*.example.org".parse().unwrap()),
            Some(super::MatchReason::WildcardInBlockedWildcard),
        );
        assert_eq!(
            super::addr_match(
                &"*.test.example.com".parse().unwrap(),
                &"*.example.org".parse().unwrap()
            ),
            None,
        );
        assert_eq!(
            super::addr_match(
                &"*.test.example.org".parse().unwrap(),
                &"*.example.org".parse().unwrap()
            ),
            Some(super::MatchReason::WildcardContainsBlockedWildcard),
        );

        assert_eq!(
            super::addr_match(
                &"http://example.com/test".parse().unwrap(),
                &"http://example.org/test".parse().unwrap()
            ),
            None,
        );
        assert_eq!(
            super::addr_match(
                &"http://example.org/test".parse().unwrap(),
                &"http://example.org/test".parse().unwrap()
            ),
            Some(super::MatchReason::URLEquals),
        );
        assert_eq!(
            super::addr_match(
                &"http://example.com/test/test2".parse().unwrap(),
                &"http://example.org/test".parse().unwrap()
            ),
            None,
        );
        assert_eq!(
            super::addr_match(
                &"http://example.org/test/test2".parse().unwrap(),
                &"http://example.org/test".parse().unwrap()
            ),
            Some(super::MatchReason::URLContainsBlockedURL),
        );
        assert_eq!(
            super::addr_match(
                &"http://example.com/".parse().unwrap(),
                &"http://example.org/test".parse().unwrap()
            ),
            None,
        );
        assert_eq!(
            super::addr_match(
                &"http://example.org/".parse().unwrap(),
                &"http://example.org/test".parse().unwrap()
            ),
            Some(super::MatchReason::URLInBlockedURL),
        );
    }
}
