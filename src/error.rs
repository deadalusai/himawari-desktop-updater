use std::error::Error;
use std::fmt::{Debug, Display, Error as FmtError, Formatter};

pub struct AppErr(String, Option<Box<dyn Error>>);

impl AppErr {
    fn from_err<E>(kind: &str, error: E) -> AppErr
    where
        E: Error + 'static,
    {
        AppErr(format!("[{}] {}", kind, error), Some(Box::new(error)))
    }

    pub fn new(kind: &str, message: &str) -> AppErr {
        AppErr(format!("[{}] {}", kind, message), None)
    }
}

impl Display for AppErr {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{}", self.0)
    }
}

impl Debug for AppErr {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        Display::fmt(self, f)
    }
}

impl Error for AppErr {
    fn description(&self) -> &str {
        &self.0
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.1 {
            Some(ref err) => Some(err.as_ref()),
            None => None,
        }
    }
}

macro_rules! impl_from_error {
    ($type:ty) => {
        impl From<$type> for AppErr {
            fn from(err: $type) -> Self {
                AppErr::from_err(stringify!($type), err)
            }
        }
    };
}

// Error conversions
impl_from_error!(std::io::Error);
impl_from_error!(std::time::SystemTimeError);
impl_from_error!(reqwest::Error);
impl_from_error!(serde_json::Error);
impl_from_error!(chrono::ParseError);
impl_from_error!(image::ImageError);
