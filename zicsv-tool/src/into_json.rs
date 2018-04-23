use std;

use failure;
use serde_json;

use zicsv;

type Records = std::collections::LinkedList<zicsv::Record>;

#[derive(Serialize)]
struct List {
    updated: zicsv::DateTime,
    records: Records,
}

fn load_records(mut reader: Box<zicsv::GenericReader>) -> Result<List, failure::Error> {
    let records: Result<Records, failure::Error> = reader.records_boxed().collect();
    Ok(List {
        updated: *reader.get_timestamp(),
        records: records?,
    })
}

pub fn into_json(reader: Box<zicsv::GenericReader>, disable_pretty: bool) -> Result<(), failure::Error> {
    let list = load_records(reader)?;

    let json_str = if disable_pretty {
        serde_json::to_string(&list)?
    } else {
        serde_json::to_string_pretty(&list)?
    };

    println!("{}", json_str);

    Ok(())
}
