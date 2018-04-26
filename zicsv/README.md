[![crates.io](https://img.shields.io/crates/v/zicsv.svg?maxAge=3600)](https://crates.io/crates/zicsv)

# zicsv

`zicsv` - Parser library for Zapret-Info CSV lists.

[Documentation on docs.rs](https://docs.rs/crate/zicsv)

Features:

* Stream parser.
* Immunity to malformed data which sometimes appears in Zapret-Info dumps.

## Usage

Add this into your `Cargo.toml`:

```toml
[dependencies]
zicsv = "*"
```

Example code:

```rust,no_run
extern crate zicsv;

fn main() {
    use zicsv::GenericReader;

    let mut reader = zicsv::Reader::from_file("dump.csv").expect("Unable to create zicsv::Reader");

    println!("Timestamp: {}", reader.get_timestamp());
    println!();

    let mut errors = false;
    for record in reader.iter() {
        match record {
            Ok(record) => println!("{}", record),

            Err(error) => {
                errors = true;
                eprintln!("ERROR: {}", error);
            },
        }
    }

    std::process::exit(if errors { 1 } else { 0 })
}
```

Download `dump.csv` manually or use `download-dump` script from this
repository before running this example.

### Running examples

```bash
git clone --branch master https://github.com/im-0/zicsv
cd zicsv
./download-dump
cargo run --package zicsv --example parse
```
