use failure;

use zicsv;

pub struct SelectOptions {
    pub ipv4: bool,
    pub ipv4_network: bool,
    pub domain: bool,
    pub wildcard_domain: bool,
    pub url: bool,
}

pub fn select(options: &SelectOptions, mut reader: Box<zicsv::GenericReader>) -> Result<(), failure::Error> {
    for record in reader.records_boxed() {
        let record = record?;

        for address in &record.addresses {
            let selected = match address {
                &zicsv::Address::IPv4(_) => options.ipv4,
                &zicsv::Address::IPv4Network(_) => options.ipv4_network,
                &zicsv::Address::DomainName(_) => options.domain,
                &zicsv::Address::WildcardDomainName(_) => options.wildcard_domain,
                &zicsv::Address::URL(_) => options.url,

                unknown => {
                    eprintln!("Warning! Unknown address type: \"{:?}\"", unknown);
                    false
                },
            };

            if selected {
                println!("{}", address);
            }
        }
    }

    Ok(())
}
