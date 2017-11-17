use std::fmt::{Debug, Display, Formatter, Error as FmtError};
use std::error::{Error};

pub struct AppErr(String, Option<Box<Error>>);

impl AppErr {
    fn new <E> (kind: &str, error: E) -> AppErr
        where E: Error + 'static
    {
        AppErr(format!("[{}] {}", kind, error), Some(Box::new(error)))
    }

    pub fn custom (kind: &str, message: &str) -> AppErr {
        AppErr(format!("[{}] {}", kind, message), None)
    }
}

impl Display for AppErr {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.description())
    }
}

impl Debug for AppErr {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        Display::fmt(self, f)
    }
}

impl Error for AppErr {
    fn description (&self) -> &str {
        &self.0
    }

    fn cause (&self) -> Option<&Error> {
        match self.1 {
            Some(ref err) => Some(err.as_ref()),
            None          => None
        }
    }
}

macro_rules! impl_from_error {
    ($type:ty) => {
        impl From<$type> for AppErr {
            fn from(err: $type) -> Self {
                AppErr::new(stringify!($type), err)
            }
        }
    }
}

// Error conversions
use std;
impl_from_error!(std::io::Error);
impl_from_error!(std::time::SystemTimeError);

use reqwest;
impl_from_error!(reqwest::Error);

use serde_json;
impl_from_error!(serde_json::Error);

use chrono;
impl_from_error!(chrono::ParseError);

use image;
impl_from_error!(image::ImageError);