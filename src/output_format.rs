use error::AppErr;
use std::fmt::{Display, Formatter, Error as FmtError};

pub enum OutputFormat {
    PNG, JPEG
}

impl OutputFormat {
    pub fn parse (s: &str) -> Result<OutputFormat, AppErr> {
        match s.trim() {
            "PNG"  | "png"  => Ok(OutputFormat::PNG),
            "JPEG" | "jpeg" => Ok(OutputFormat::JPEG),
            _               => Err(AppErr::custom("output-format", "Invalid image format, use JPEG or PNG"))
        }
    }
}

impl Display for OutputFormat {
    fn fmt (&self, f: &mut Formatter) -> Result<(), FmtError> {
        let s = match *self {
            OutputFormat::PNG  => "png",
            OutputFormat::JPEG => "jpeg",
        };
        write!(f, "{}", s)
    }
}