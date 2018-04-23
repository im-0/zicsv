use std;

use failure;
use serde;
use serde_json;

use zicsv;

use print_err;

pub struct RecordsSerializer<'a> {
    errors: bool,
    records: Box<Iterator<Item = Result<zicsv::Record, failure::Error>> + 'a>,
}

mod serialize_records {
    use super::*;

    pub fn serialize<'a, S>(value: &std::cell::RefCell<RecordsSerializer<'a>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use std::ops::DerefMut;

        use serde::ser::SerializeSeq;

        let mut records_serializer = value.borrow_mut();
        let &mut RecordsSerializer {
            ref mut errors,
            ref mut records,
        } = records_serializer.deref_mut();

        let mut seq = serializer.serialize_seq(None)?;
        for record in records {
            match record {
                Ok(record) => seq.serialize_element(&record)?,
                Err(error) => {
                    *errors = true;
                    print_err::print_error(&error);
                },
            }
        }
        seq.end()
    }
}

#[derive(Serialize)]
struct List<'a> {
    updated: zicsv::DateTime,
    #[serde(with = "serialize_records")]
    records: std::cell::RefCell<RecordsSerializer<'a>>,
}

pub fn into_json(mut reader: Box<zicsv::GenericReader>, disable_pretty: bool) -> Result<(), failure::Error> {
    let updated = *reader.get_timestamp();
    let records = reader.iter();

    let list = List {
        updated,
        records: std::cell::RefCell::new(RecordsSerializer { errors: false, records }),
    };

    if disable_pretty {
        serde_json::to_writer(std::io::stdout(), &list)?;
    } else {
        serde_json::to_writer_pretty(std::io::stdout(), &list)?;
    };

    Ok(())
}
