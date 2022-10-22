use std::fmt::Display;

#[derive(Clone)]
pub struct Margins {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

#[derive(Clone)]
pub struct MarginsValueParser;

impl clap::builder::TypedValueParser for MarginsValueParser {
    type Value = Margins;
    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        use clap::error::{Error, ErrorKind};
        match Margins::try_parse(value.to_string_lossy().as_ref()) {
            Some(m) => Ok(m),
            None => Err(Error::raw(
                ErrorKind::InvalidValue,
                "Use format TOP[,RIGHT][,BOTTOM][,LEFT]",
            )),
        }
    }
}

impl Margins {
    pub fn try_parse(input: &str) -> Option<Margins> {
        let mut parts = input.split(",").map(|s| s.trim()).map(|n| n.parse::<u32>());

        let top = parts.next().unwrap_or(Ok(0)).ok()?;
        let right = parts.next().unwrap_or(Ok(top)).ok()?;
        let bottom = parts.next().unwrap_or(Ok(top)).ok()?;
        let left = parts.next().unwrap_or(Ok(right)).ok()?;

        if parts.next().is_some() {
            return None;
        }

        Some(Margins {
            top,
            right,
            bottom,
            left,
        })
    }
}

impl Display for Margins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {}, {}, {}",
            self.top, self.right, self.bottom, self.left
        )
    }
}

impl Default for Margins {
    fn default() -> Margins {
        Margins {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        }
    }
}
