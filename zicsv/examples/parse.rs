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
