use std::path::Path;
use crate::error::AppErr;

pub fn set_wallpaper(_image_path: &Path) -> Result<(), AppErr> {
    // TODO: Linux/OSX versions of set_wallpaper?
    warn!("Setting the wallpaper is not supported on this platform");
    Ok(())
}
