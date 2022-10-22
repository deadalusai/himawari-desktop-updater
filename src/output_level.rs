use std::fmt::{Display, Error as FmtError, Formatter};

#[derive(Clone)]
pub struct OutputLevel(u32);

#[derive(Clone)]
pub struct OutputLevelValueParser;

impl clap::builder::TypedValueParser for OutputLevelValueParser {
    type Value = OutputLevel;
    fn parse_ref(&self, _cmd: &clap::Command, _arg: Option<&clap::Arg>, value: &std::ffi::OsStr) -> Result<Self::Value, clap::Error> {
        use clap::error::{Error, ErrorKind};
        match value.to_string_lossy().as_ref().trim().parse::<u32>() {
            Ok(n) if n == 4 || n == 8 || n == 16 || n == 20 => Ok(OutputLevel(n)),
            _ => Err(Error::raw(ErrorKind::InvalidValue, "Invalid level, use 4, 8, 16 or 20")),
        }
    }
}

impl OutputLevel {
    pub fn to_level(&self) -> u32 {
        self.0
    }
}

impl Display for OutputLevel {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        let &OutputLevel(ref n) = self;
        write!(f, "{}", n)
    }
}

impl Default for OutputLevel {
    fn default() -> OutputLevel {
        OutputLevel(8)
    }
}
