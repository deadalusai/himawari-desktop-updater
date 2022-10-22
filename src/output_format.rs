use std::fmt::{Display, Error as FmtError, Formatter};

#[derive(Clone)]
pub enum OutputFormat {
    PNG,
    JPEG,
}

#[derive(Clone)]
pub struct OutputFormatValueParser;

impl clap::builder::TypedValueParser for OutputFormatValueParser {
    type Value = OutputFormat;
    fn parse_ref(&self, _cmd: &clap::Command, _arg: Option<&clap::Arg>, value: &std::ffi::OsStr) -> Result<Self::Value, clap::Error> {
        use clap::error::{Error, ErrorKind};
        match value.to_string_lossy().as_ref().trim() {
            "PNG" | "png" => Ok(OutputFormat::PNG),
            "JPEG" | "jpeg" => Ok(OutputFormat::JPEG),
            _ => Err(Error::raw(ErrorKind::InvalidValue, "Invalid image format, use JPEG or PNG")),
        }
    }
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        let s = match *self {
            OutputFormat::PNG => "png",
            OutputFormat::JPEG => "jpeg",
        };
        write!(f, "{}", s)
    }
}

impl Default for OutputFormat {
    fn default() -> OutputFormat {
        OutputFormat::JPEG
    }
}
