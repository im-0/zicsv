use std;

use failure;
use serde;
use serde_json;

use zicsv;

use print_err;

pub struct RecordsSerializer<'a> {
    n_errors: usize,
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
            ref mut n_errors,
            ref mut records,
        } = records_serializer.deref_mut();

        let mut seq = serializer.serialize_seq(None)?;
        for record in records {
            match record {
                Ok(record) => seq.serialize_element(&record)?,
                Err(error) => {
                    *n_errors += 1;
                    print_err::print_error(&error);
                }
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

pub fn into_json<StreamWriter>(
    mut reader: Box<zicsv::GenericReader>,
    writer: &mut StreamWriter,
    disable_pretty: bool,
) -> Result<(), failure::Error>
where
    StreamWriter: std::io::Write,
{
    let updated = *reader.get_timestamp();
    let records = reader.iter();

    let list = List {
        updated,
        records: std::cell::RefCell::new(RecordsSerializer { n_errors: 0, records }),
    };

    if disable_pretty {
        serde_json::to_writer(writer, &list)?;
    } else {
        serde_json::to_writer_pretty(writer, &list)?;
    };

    let n_errors = list.records.borrow().n_errors;
    ensure!(n_errors == 0, "{} errors occur while reading list", n_errors);
    Ok(())
}
