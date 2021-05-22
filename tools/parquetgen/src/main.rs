use parquet::{
    column::writer::ColumnWriter,
    data_type::ByteArray,
    file::{
        properties::WriterProperties,
        writer::{FileWriter, SerializedFileWriter},
    },
    schema::parser::parse_message_type,
};
use std::env;
use std::error;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process;
use std::rc::Rc;

enum Column {
    Integer(Vec<i32>),
    Varchar(Vec<&'static str>),
}

#[derive(Debug)]
struct Error {
    message: String,
}

impl Error {
    fn new(msg: String) -> Box<Self> {
        Box::new(Self {
            message: msg.to_string(),
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.message
    }
}

fn write_table(path: std::path::PathBuf, schema: &'static str, data: Vec<Column>) {
    let filename = path.file_name().unwrap().to_str().unwrap_or("?");
    print!("{:.<24} ", &filename);

    match || -> Result<(), Box<dyn std::error::Error>> {
        fs::remove_file(&path).ok();
        let schema = Rc::new(parse_message_type(&schema)?);
        let props = Rc::new(WriterProperties::builder().build());
        let file = fs::File::create(&path)?;

        let mut writer = SerializedFileWriter::new(file, schema, props)?;
        let mut row_group_writer = writer.next_row_group()?;
        let mut column_id = 0;

        while let Some(mut col_writer) = row_group_writer.next_column()? {
            if column_id >= data.len() {
                return Err(Error::new(format!(
                    "Schema contains more columns than provided. (expected {})",
                    data.len()
                )));
            }
            match (&data[column_id], &mut col_writer) {
                (Column::Integer(ref v), ColumnWriter::Int32ColumnWriter(ref mut w)) => {
                    w.write_batch(&v, None, None)?;
                }
                (Column::Varchar(ref v), ColumnWriter::ByteArrayColumnWriter(ref mut w)) => {
                    let buffer: Vec<ByteArray> = v.iter().map(|x| ByteArray::from(*x)).collect();
                    w.write_batch(&buffer, None, None)?;
                }
                (_, _) => {
                    return Err(Error::new(format!(
                        "Type mismatch for column {}",
                        column_id
                    )));
                }
            }
            row_group_writer.close_column(col_writer)?;
            column_id += 1;
        }
        writer.close_row_group(row_group_writer)?;
        writer.close()?;

        Ok(())
    }() {
        Ok(_) => println!("OK"),
        Err(e) => println!("ERR\n{}", e),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: pkg_uni_schema <dir>");
        process::exit(1);
    }
    let out_dir = Path::new(&args[1]);
    let out_dir_str = out_dir.to_str().unwrap_or("?");
    let out_metadata = fs::metadata(out_dir);
    if out_metadata.is_err() || !out_metadata.unwrap().is_dir() {
        println!("Invalid output directory: {}", out_dir_str);
        process::exit(1);
    }

    write_table(
        out_dir.join("professoren.parquet"),
        "
        message schema {
            required int32 PersNr;
            required byte_array PersNr (utf8);
            required byte_array Rang (utf8);
            required int32 Raum;
        }
    ",
        vec![
            Column::Integer(vec![2125, 2126, 2127, 2133, 2134, 2136, 2137]),
            Column::Varchar(vec![
                "Sokrates",
                "Russel",
                "Kopernikus",
                "Popper",
                "Augustinus",
                "Curie",
                "Kant",
            ]),
            Column::Varchar(vec!["C4", "C4", "C3", "C3", "C3", "C4", "C4"]),
            Column::Integer(vec![226, 232, 310, 52, 309, 36, 7]),
        ],
    );

    write_table(
        out_dir.join("studenten.parquet"),
        "
        message schema {
            required int32 MatrNr;
            required byte_array Name (utf8);
            required int32 Semester;
        }
    ",
        vec![
            Column::Integer(vec![24002, 25403, 26120, 26830, 27550, 28106, 29120, 29555]),
            Column::Varchar(vec![
                "Xenokrates",
                "Jonas",
                "Fichte",
                "Aristoxenos",
                "Schopenhauer",
                "Carnap",
                "Theophrastos",
                "Feuerbach",
            ]),
            Column::Integer(vec![18, 12, 10, 8, 6, 3, 2, 2]),
        ],
    );

    write_table(
        out_dir.join("vorlesungen.parquet"),
        "
        message schema {
            required int32 VorlNr;
            required byte_array Titel (utf8);
            required int32 SWS;
            required int32 gelesenVon;
        }
    ",
        vec![
            Column::Integer(vec![
                5001, 5041, 5043, 5049, 4052, 5052, 5216, 5259, 5022, 4630,
            ]),
            Column::Varchar(vec![
                "Grundzüge",
                "Ethik",
                "Erkenntnistheorie",
                "Mäeutik",
                "Logik",
                "Wissenschaftstheorie",
                "Bioethik",
                "Der Wiener Kreis",
                "Glaube und Wissen",
                "Die 3 Kritiken",
            ]),
            Column::Integer(vec![4, 4, 3, 2, 4, 3, 2, 2, 2, 4]),
            Column::Integer(vec![
                2137, 2125, 2126, 2125, 2125, 2126, 2126, 2133, 2134, 2137,
            ]),
        ],
    );

    write_table(
        out_dir.join("vorraussetzen.parquet"),
        "
        message schema {
            required int32 Vorgaenger;
            required int32 Nachfolger;
        }
    ",
        vec![
            Column::Integer(vec![5001, 5001, 5001, 5041, 5043, 5041, 5052]),
            Column::Integer(vec![5041, 5043, 5049, 5216, 5052, 5052, 5259]),
        ],
    );

    write_table(
        out_dir.join("hoeren.parquet"),
        "
        message schema {
            required int32 MatrNr;
            required int32 VorlNr;
        }
    ",
        vec![
            Column::Integer(vec![
                26120, 27550, 27550, 28106, 28106, 28106, 28106, 29120, 29120, 29120, 29555, 25403,
            ]),
            Column::Integer(vec![
                5001, 5001, 4052, 5041, 5052, 5216, 5259, 5001, 5041, 5049, 5022, 5022,
            ]),
        ],
    );

    write_table(
        out_dir.join("pruefen.parquet"),
        "
        message schema {
            required int32 MatrNr;
            required int32 VorlNr;
            required int32 PersNr;
            required int32 Note;
        }
    ",
        vec![
            Column::Integer(vec![28106, 25403, 27550]),
            Column::Integer(vec![5001, 5041, 4630]),
            Column::Integer(vec![2126, 2125, 2137]),
            Column::Integer(vec![1, 2, 2]),
        ],
    );

    write_table(
        out_dir.join("assistenten.parquet"),
        "
        message schema {
            required int32 PersNr;
            required byte_array Name (utf8);
            required byte_array Fachgebiet (utf8);
            required int32 Boss;
        }
    ",
        vec![
            Column::Integer(vec![3002, 3003, 3004, 3005, 3006, 3007]),
            Column::Varchar(vec![
                "Platon",
                "Aristoteles",
                "Wittgenstein",
                "Rhetikus",
                "Newton",
                "Spinoza",
            ]),
            Column::Varchar(vec![
                "Ideenlehre",
                "Syllogistik",
                "Sprachteorie",
                "Planetenbewegung",
                "Keplersche Gesetze",
                "Gott und Natur",
            ]),
            Column::Integer(vec![2125, 2125, 2126, 2127, 2127, 2126]),
        ],
    );
}
