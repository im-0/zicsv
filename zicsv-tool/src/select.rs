use std;

use failure;

use zicsv;

use print_err;

pub struct SelectOptions {
    pub ipv4: bool,
    pub ipv4_network: bool,
    pub domain: bool,
    pub wildcard_domain: bool,
    pub url: bool,
}

pub fn select<StreamWriter>(
    options: &SelectOptions,
    mut reader: Box<zicsv::GenericReader>,
    writer: &mut StreamWriter,
) -> Result<(), failure::Error>
where
    StreamWriter: std::io::Write,
{
    let mut n_errors = 0usize;

    for record in reader.iter() {
        match record {
            Ok(record) => for address in &record.addresses {
                let selected = match *address {
                    zicsv::Address::IPv4(_) => options.ipv4,
                    zicsv::Address::IPv4Network(_) => options.ipv4_network,
                    zicsv::Address::DomainName(_) => options.domain,
                    zicsv::Address::WildcardDomainName(_) => options.wildcard_domain,
                    zicsv::Address::URL(_) => options.url,

                    // Do nothing on unknown type of address.
                    _ => false,
                };

                if selected {
                    writeln!(writer, "{}", address)?;
                }
            },

            Err(error) => {
                n_errors += 1;
                print_err::print_error(&error);
            }
        }
    }

    ensure!(n_errors == 0, "{} errors occur while reading list", n_errors);
    Ok(())
}
