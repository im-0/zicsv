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

extern crate ipnet;

#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

extern crate pretty_env_logger;

extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate serde_json;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;

extern crate trust_dns_proto;
extern crate trust_dns_resolver;
extern crate url;

extern crate zicsv;

mod into_json;
mod print_err;
mod search;
mod select;

#[derive(Debug)]
pub enum OutputFormat {
    HumanReadable,
    PrettyJSON,
    JSON,
}

// TODO: Support different output formats for "select" and "updated".
impl OutputFormat {
    fn variants() -> Vec<&'static str> {
        vec!["human-readable", "pretty-json", "json"]
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human-readable" => Ok(OutputFormat::HumanReadable),
            "pretty-json" => Ok(OutputFormat::PrettyJSON),
            "json" => Ok(OutputFormat::JSON),

            unknown_format => Err(format_err!("Unknown output format: \"{}\"", unknown_format)),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            formatter,
            "{}",
            match *self {
                OutputFormat::HumanReadable => "human-readable",
                OutputFormat::PrettyJSON => "pretty-json",
                OutputFormat::JSON => "json",
            }
        )
    }
}

#[derive(StructOpt, Debug)]
#[structopt(raw(global_settings = "&[structopt::clap::AppSettings::ColorAuto]"))]
enum Command {
    #[structopt(name = "into-json", about = "Convert into json format")]
    IntoJson {
        #[structopt(short = "P", long = "disable-pretty", help = "Disable pretty-printing")]
        disable_pretty: bool,
    },

    #[structopt(name = "select", about = "Print selected types of blocked addresses")]
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

    #[structopt(name = "search", about = "Search blocked addresses")]
    Search {
        #[structopt(name = "OUTPUT FORMAT", short = "O", long = "output-format", default_value = "human-readable",
                    raw(possible_values = "&OutputFormat::variants()"))]
        output_format: OutputFormat,

        #[structopt(name = "ADDRESS")]
        addresses: Vec<String>,
    },
}

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short = "i", long = "input", help = "Read from file instead of stdin")]
    input_path: Option<String>,

    #[structopt(short = "o", long = "output", help = "Write into file instead of stdout")]
    output_path: Option<String>,

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

fn create_writer<'a>(
    options: &Options,
    stdout: &'a mut std::io::Stdout,
) -> Result<Box<std::io::Write + 'a>, failure::Error> {
    Ok(if let Some(output_path) = options.output_path.as_ref() {
        Box::new(std::io::BufWriter::new(std::fs::File::create(output_path)?))
    } else {
        // Adding another buffer on top of stdout effectively disables line-buffering of rust's stdout which makes things
        // a bit faster.
        Box::new(std::io::BufWriter::new(stdout.lock()))
    })
}

fn real_main() -> Result<(), failure::Error> {
    use std::io::Write;

    use structopt::StructOpt;

    pretty_env_logger::try_init()?;

    // TODO: Also generate auto-completion scripts.
    let options = Options::from_args();

    let reader = create_reader(&options)?;

    let mut stdout = std::io::stdout();
    let mut writer = create_writer(&options, &mut stdout)?;

    match options.command {
        Command::IntoJson { disable_pretty } => into_json::into_json(reader, &mut writer, disable_pretty)?,

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

            select::select(&sopts, reader, &mut writer)?
        }

        Command::Updated => writeln!(writer, "{}", reader.get_timestamp())?,

        Command::Search {
            addresses,
            output_format,
        } => {
            ensure!(!addresses.is_empty(), "At least one address should be specified");

            search::search(&addresses, reader, &mut writer, &output_format)?
        }
    }
    writer.flush()?;

    Ok(())
}

fn main() {
    let rc = real_main().map(|_| 0).unwrap_or_else(|error| {
        print_err::print_error(&error);
        1
    });
    std::process::exit(rc)
}
