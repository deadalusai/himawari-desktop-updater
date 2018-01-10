use error::AppErr;
use std::fmt::{Display, Formatter, Error as FmtError};

pub struct OutputLevel(u32);

impl OutputLevel {
    pub fn parse (s: &str) -> Result<OutputLevel, AppErr> {
        match s.trim().parse::<u32>() {
            Ok(n) if n == 4 || n == 8 || n == 16 || n == 20 => Ok(OutputLevel(n)),
            _ => Err(AppErr::custom("output-level", "Invalid level, use 4, 8, 16 or 20"))
        }
    }

    pub fn to_level(&self) -> u32 {
        self.0
    }
}

impl Display for OutputLevel {
    fn fmt (&self, f: &mut Formatter) -> Result<(), FmtError> {
        let &OutputLevel(ref n) = self;
        write!(f, "{}", n)
    }
}

impl Default for OutputLevel {
    fn default () -> OutputLevel {
        OutputLevel(8)
    }
}