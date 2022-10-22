use crate::error::AppErr;

pub struct Margins {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Margins {
    pub fn parse(input: &str) -> Result<Margins, AppErr> {
        let fail = || AppErr::new("margins", "Use format TOP[,RIGHT][,BOTTOM][,LEFT]");

        let mut parts = input
            .split(",")
            .map(|s| s.trim())
            .map(|n| n.parse::<u32>().map_err(|_| fail()));

        let top = parts.next().unwrap_or(Ok(0))?;
        let right = parts.next().unwrap_or(Ok(top))?;
        let bottom = parts.next().unwrap_or(Ok(top))?;
        let left = parts.next().unwrap_or(Ok(right))?;

        if parts.next().is_some() {
            return Err(fail());
        }

        Ok(Margins {
            top,
            right,
            bottom,
            left,
        })
    }

    pub fn empty() -> Margins {
        Margins {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        }
    }
}
