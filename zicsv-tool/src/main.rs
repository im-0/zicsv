#![forbid(unsafe_code)]
#![warn(unused_results)]
#![cfg_attr(feature = "cargo-clippy", warn(empty_line_after_outer_attr))]
#![cfg_attr(feature = "cargo-clippy", warn(filter_map))]
#![cfg_attr(feature = "cargo-clippy", warn(if_not_else))]
#![cfg_attr(feature = "cargo-clippy", warn(mut_mut))]
#![cfg_attr(feature = "cargo-clippy", warn(non_ascii_literal))]
#![cfg_attr(feature = "cargo-clippy", warn(option_map_unwrap_or))]
#![cfg_attr(feature = "cargo-clippy", warn(option_map_unwrap_or_else))]
#![cfg_attr(feature = "cargo-clippy", warn(single_match_else))]
#![cfg_attr(feature = "cargo-clippy", warn(wrong_pub_self_convention))]
#![cfg_attr(feature = "cargo-clippy", warn(use_self))]
#![cfg_attr(feature = "cargo-clippy", warn(used_underscore_binding))]
#![cfg_attr(feature = "cargo-clippy", warn(else_if_without_else))]

#[macro_use]
extern crate failure;

extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate serde_json;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;

extern crate zicsv;

mod into_json;
mod print_err;
mod select;

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "into-json", about = "Convert into json format")]
    IntoJson {
        #[structopt(short = "P", long = "disable-pretty", help = "Disable pretty-printing")]
        disable_pretty: bool,
    },

    #[structopt(name = "select", about = "Print selected types of blocked addresses into stdout")]
    Select {
        #[structopt(short = "4", long = "ipv4", help = "IPv4 addresses")]
        ipv4: bool,

        #[structopt(short = "n", long = "ipv4-network", help = "IPv4 networks")]
        ipv4_network: bool,

        #[structopt(short = "d", long = "domain", help = "Domain names")]
        domain: bool,

        #[structopt(short = "w", long = "wildcard-domain", help = "Wildcard domain names")]
        wildcard_domain: bool,

        #[structopt(short = "u", long = "url", help = "URLs")]
        url: bool,
    },

    #[structopt(name = "updated", about = "Print date of last update")]
    Updated,
}

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short = "i", long = "input", help = "Read from file instead of stdin")]
    input_path: Option<String>,

    #[structopt(subcommand)]
    command: Command,
}

fn create_reader(options: &Options) -> Result<Box<zicsv::GenericReader>, failure::Error> {
    Ok(if let Some(input_path) = options.input_path.as_ref() {
        Box::new(zicsv::Reader::from_file(input_path)?)
    } else {
        Box::new(zicsv::Reader::from_reader(std::io::stdin())?)
    })
}

fn real_main() -> Result<(), failure::Error> {
    use structopt::StructOpt;

    let options = Options::from_args();

    let reader = create_reader(&options)?;

    match options.command {
        Command::IntoJson { disable_pretty, .. } => into_json::into_json(reader, disable_pretty),

        Command::Select {
            ipv4,
            ipv4_network,
            domain,
            wildcard_domain,
            url,
        } => {
            let sopts = select::SelectOptions {
                ipv4,
                ipv4_network,
                domain,
                wildcard_domain,
                url,
            };
            ensure!(
                sopts.ipv4 || sopts.ipv4_network || sopts.domain || sopts.wildcard_domain || sopts.url,
                "At least one selection should be specified"
            );

            select::select(&sopts, reader)
        },

        Command::Updated => {
            println!("{}", reader.get_timestamp());
            Ok(())
        },
    }
}

fn main() {
    let rc = real_main().map(|_| 0).unwrap_or_else(|error| {
        print_err::print_error(&error);
        1
    });
    std::process::exit(rc)
}
