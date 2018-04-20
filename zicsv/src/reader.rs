use std;

use chrono;
use csv;
use encoding;
use failure;
use ipnet;
use url;

use types;

type StringRecord = (String, String, String, String, String, String);

pub trait GenericReader {
    /// Date of last update of this list.
    fn get_timestamp(&self) -> &types::DateTime;

    /// Iterate over records using generic iterator.
    fn records_boxed<'a>(&'a mut self) -> Box<Iterator<Item = Result<types::Record, failure::Error>> + 'a>;
}

pub struct Reader<StreamReader>
where
    StreamReader: std::io::BufRead,
{
    updated: types::DateTime,
    csv_reader: csv::Reader<StreamReader>,
}

impl<StreamReader> Reader<StreamReader>
where
    StreamReader: std::io::BufRead,
{
    fn parse_update_datetime(reader: &mut StreamReader) -> Result<types::DateTime, failure::Error> {
        let mut first_line = String::new();
        let _ = reader.read_line(&mut first_line)?;

        let space_pos = first_line.find(':').ok_or_else(|| {
            format_err!(
                "No ':' (should be in format \"Updated: $DATE_TIME\"): \"{}\"",
                first_line
            )
        })?;
        let (_, updated) = first_line.split_at(space_pos + 1);
        let updated = updated.trim();

        let updated = chrono::DateTime::parse_from_str(updated, "%Y-%m-%d %H:%M:%S %z").map_err(|error| {
            format_err!(
                "Invalid date and time: \"{}\" (\"{}\": {})",
                first_line,
                updated,
                error
            )
        })?;
        Ok(updated.naive_utc())
    }

    /// Parse data from buffered reader.
    pub fn from_buf_reader(mut reader: StreamReader) -> Result<Self, failure::Error> {
        Ok(Self {
            updated: Self::parse_update_datetime(&mut reader).map_err(|error| error.context("Line 1"))?,
            csv_reader: csv::Reader::from_reader(reader)
                .delimiter(b';')
                .has_headers(false)
                .flexible(true),
        })
    }

    /// Iterate over records.
    pub fn records(&mut self) -> Records<StreamReader> {
        Records {
            csv_records: self.csv_reader.byte_records(),
            line_n: 1,
        }
    }
}

impl<UnbufferedReader> Reader<std::io::BufReader<UnbufferedReader>>
where
    UnbufferedReader: std::io::Read,
{
    /// Parse data from normal (not buffered) reader.
    pub fn from_reader(reader: UnbufferedReader) -> Result<Self, failure::Error> {
        Self::from_buf_reader(std::io::BufReader::new(reader))
    }
}

impl Reader<std::io::BufReader<std::fs::File>> {
    fn from_file_no_context<Path: AsRef<std::path::Path>>(path: Path) -> Result<Self, failure::Error> {
        Self::from_reader(std::fs::File::open(path)?)
    }

    /// Parse data from file specified by path.
    pub fn from_file<Path: AsRef<std::path::Path>>(path: Path) -> Result<Self, failure::Error> {
        // TODO: Provide file name as context for Records::next().
        let path_str = format!("{}", path.as_ref().to_string_lossy());
        Self::from_file_no_context(path).map_err(|error| error.context(format!("File \"{}\"", path_str)).into())
    }
}

impl<StreamReader> GenericReader for Reader<StreamReader>
where
    StreamReader: std::io::BufRead,
{
    fn get_timestamp(&self) -> &types::DateTime {
        &self.updated
    }

    fn records_boxed<'a>(&'a mut self) -> Box<Iterator<Item = Result<types::Record, failure::Error>> + 'a> {
        Box::new(self.records())
    }
}

pub struct Records<'a, StreamReader: 'a>
where
    StreamReader: std::io::BufRead,
{
    csv_records: csv::ByteRecords<'a, StreamReader>,
    line_n: u64,
}

impl<'a, StreamReader: 'a> Records<'a, StreamReader>
where
    StreamReader: std::io::BufRead,
{
    fn str_from_cp1251(raw: &[u8]) -> Result<String, failure::Error> {
        use encoding::Encoding;

        encoding::all::WINDOWS_1251
            .decode(raw, encoding::DecoderTrap::Strict)
            .map_err(|error| format_err!("Invalid CP1251 string ({})", error))
    }

    fn str_rec_from_cp1251(raw_vec: &[Vec<u8>]) -> Result<StringRecord, failure::Error> {
        ensure!(
            raw_vec.len() == 6,
            "Invalid number of fields: {} != 6",
            raw_vec.len()
        );

        Ok((
            Self::str_from_cp1251(&raw_vec[0])?,
            Self::str_from_cp1251(&raw_vec[1])?,
            Self::str_from_cp1251(&raw_vec[2])?,
            Self::str_from_cp1251(&raw_vec[3])?,
            Self::str_from_cp1251(&raw_vec[4])?,
            Self::str_from_cp1251(&raw_vec[5])?,
        ))
    }

    fn parse_for_each<ParseFn>(addr_str: &str, delim: &str, mut func: ParseFn) -> Result<(), failure::Error>
    where
        ParseFn: FnMut(&str) -> Result<(), failure::Error>,
    {
        for part in addr_str.split(delim) {
            let part = part.trim();
            if !part.is_empty() {
                func(part)?;
            }
        }

        Ok(())
    }

    fn parse_ipv4_addresses(addr_str: &str, addresses: &mut types::Addresses) -> Result<(), failure::Error> {
        use std::str::FromStr;

        Self::parse_for_each(addr_str, "|", |part| {
            if part.contains('/') {
                addresses.push(types::Address::IPv4Network(ipnet::Ipv4Net::from_str(part)?));
            } else {
                addresses.push(types::Address::IPv4(std::net::Ipv4Addr::from_str(part)?));
            }

            Ok(())
        })
    }

    fn parse_domain_name(addr_str: &str, addresses: &mut types::Addresses) -> Result<(), failure::Error> {
        Self::parse_for_each(addr_str, "|", |part| {
            {
                if part.starts_with('*') {
                    addresses.push(types::Address::WildcardDomainName(part.into()));
                } else {
                    addresses.push(types::Address::DomainName(part.into()));
                }
            }

            Ok(())
        })
    }

    fn parse_url(addr_str: &str, addresses: &mut types::Addresses) -> Result<(), failure::Error> {
        use std::str::FromStr;

        // We are using " | " as a delimiter because URL itself may contain '|'.
        Self::parse_for_each(addr_str, " | ", |part| {
            {
                addresses.push(types::Address::URL(url::Url::from_str(part)?));
            }

            Ok(())
        })
    }

    fn parse_document_date(date_str: &str) -> Result<types::Date, failure::Error> {
        Ok(types::Date::parse_from_str(date_str.trim(), "%Y-%m-%d")?)
    }

    fn parse_record(record: &StringRecord) -> Result<types::Record, failure::Error> {
        let mut addresses = types::Addresses::new();

        Self::parse_ipv4_addresses(&record.0, &mut addresses)?;
        Self::parse_domain_name(&record.1, &mut addresses)?;
        Self::parse_url(&record.2, &mut addresses)?;

        Ok(types::Record {
            addresses,
            organization: record.3.trim().into(),
            document_id: record.4.trim().into(),
            document_date: Self::parse_document_date(&record.5)?,

            __may_be_extended: (),
        })
    }
}

impl<'a, StreamReader: 'a> Iterator for Records<'a, StreamReader>
where
    StreamReader: std::io::BufRead,
{
    type Item = Result<types::Record, failure::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.line_n += 1;

        self.csv_records.next().map(|csv_result| -> Self::Item {
            csv_result
                .map_err(|csv_err| csv_err.into())
                .and_then(|raw_record| Self::str_rec_from_cp1251(&raw_record))
                .and_then(|str_record| Self::parse_record(&str_record))
                .map_err(|error| error.context(format!("Line {}", self.line_n)).into())
        })
    }
}

#[cfg(test)]
mod tests {
    use std;

    use chrono;
    use failure;

    use types;

    type Reader<'a> = super::Reader<std::io::BufReader<std::io::Cursor<&'a str>>>;

    fn from_str(data: &str) -> Result<Reader, failure::Error> {
        let stream = std::io::Cursor::new(data);
        super::Reader::from_reader(stream)
    }

    #[test]
    fn parse_valid_timestamp() {
        use reader::GenericReader;

        let reader = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\
             ",
        ).unwrap();
        assert_eq!(
            *reader.get_timestamp(),
            chrono::NaiveDate::from_ymd(2017, 11, 29).and_hms(13, 34, 56)
        );

        let reader = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;;;2017-01-02\n\
             ",
        ).unwrap();
        assert_eq!(
            *reader.get_timestamp(),
            chrono::NaiveDate::from_ymd(2017, 11, 29).and_hms(13, 34, 56)
        );
    }

    #[test]
    fn parse_invalid_timestamp() {
        let reader = from_str("");
        assert!(reader.is_err());

        // No ':'.
        let reader = from_str(
            "\
             test\
             ",
        );
        assert!(reader.is_err());

        // Invalid date/time format.
        let reader = from_str(
            "\
             Updated: 2017-11-29 12:34:56\
             ",
        );
        assert!(reader.is_err());
    }

    #[test]
    fn parse_no_records() {
        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ",
        ).unwrap()
            .records()
            .next();
        assert!(record.is_none());
    }

    #[test]
    fn parse_valid_record() {
        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;;;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap()
            .unwrap();
        assert!(record.addresses.is_empty());
        assert!(record.organization.is_empty());
        assert!(record.document_id.is_empty());
        assert_eq!(
            record.document_date,
            chrono::NaiveDate::from_ymd(2017, 01, 02)
        );

        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;org string;id string;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap()
            .unwrap();
        assert!(record.addresses.is_empty());
        assert_eq!(record.organization, "org string");
        assert_eq!(record.document_id, "id string");
        assert_eq!(
            record.document_date,
            chrono::NaiveDate::from_ymd(2017, 01, 02)
        );

        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;\"org string\";id string;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap()
            .unwrap();
        assert!(record.addresses.is_empty());
        assert_eq!(record.organization, "org string");
        assert_eq!(record.document_id, "id string");
        assert_eq!(
            record.document_date,
            chrono::NaiveDate::from_ymd(2017, 01, 02)
        );

        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;\"org;string\";id string;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap()
            .unwrap();
        assert!(record.addresses.is_empty());
        assert_eq!(record.organization, "org;string");
        assert_eq!(record.document_id, "id string");
        assert_eq!(
            record.document_date,
            chrono::NaiveDate::from_ymd(2017, 01, 02)
        );

        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             1.2.3.4;example.com;http://example.com;;;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap()
            .unwrap();
        let addresses = vec![
            types::Address::IPv4("1.2.3.4".parse().unwrap()),
            types::Address::DomainName("example.com".into()),
            types::Address::URL("http://example.com".parse().unwrap()),
        ];
        assert_eq!(record.addresses, addresses);
        assert!(record.organization.is_empty());
        assert!(record.document_id.is_empty());
        assert_eq!(
            record.document_date,
            chrono::NaiveDate::from_ymd(2017, 01, 02)
        );

        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             1.2.3.4|1.2.3.0/24;example.com|*.example.com;http://example.com?test=x|y;;;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap()
            .unwrap();
        let addresses = vec![
            types::Address::IPv4("1.2.3.4".parse().unwrap()),
            types::Address::IPv4Network("1.2.3.0/24".parse().unwrap()),
            types::Address::DomainName("example.com".into()),
            types::Address::WildcardDomainName("*.example.com".into()),
            types::Address::URL("http://example.com?test=x|y".parse().unwrap()),
        ];
        assert_eq!(record.addresses, addresses);
        assert!(record.organization.is_empty());
        assert!(record.document_id.is_empty());
        assert_eq!(
            record.document_date,
            chrono::NaiveDate::from_ymd(2017, 01, 02)
        );

        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             1.2.3.4 | 1.2.3.0/24;example.com | \
             *.example.com;http://example.com?test=x | http://example.com?test=y;;;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap()
            .unwrap();
        let addresses = vec![
            types::Address::IPv4("1.2.3.4".parse().unwrap()),
            types::Address::IPv4Network("1.2.3.0/24".parse().unwrap()),
            types::Address::DomainName("example.com".into()),
            types::Address::WildcardDomainName("*.example.com".into()),
            types::Address::URL("http://example.com?test=x".parse().unwrap()),
            types::Address::URL("http://example.com?test=y".parse().unwrap()),
        ];
        assert_eq!(record.addresses, addresses);
        assert!(record.organization.is_empty());
        assert!(record.document_id.is_empty());
        assert_eq!(
            record.document_date,
            chrono::NaiveDate::from_ymd(2017, 01, 02)
        );
    }

    #[test]
    fn parse_invalid_record() {
        // Too many columns.
        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;;;;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap();
        assert!(record.is_err());

        // Not enough columns.
        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap();
        assert!(record.is_err());

        // No date.
        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;;;\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap();
        assert!(record.is_err());

        // Invalid date format.
        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;;;;test\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap();
        assert!(record.is_err());

        // Invalid IPv4 address.
        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             invalid;;;;;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap();
        assert!(record.is_err());

        // Invalid URL.
        let record = from_str(
            "\
             Updated: 2017-11-29 12:34:56 -0100\n\
             ;;invalid;;;2017-01-02\n\
             ",
        ).unwrap()
            .records()
            .next()
            .unwrap();
        assert!(record.is_err());
    }
}
