use std::path::Path;
use log::{info};
use crate::error::AppErr;

pub fn set_wallpaper(image_path: &Path) -> Result<(), AppErr> {
    // Set registry flags to control wallpaper style
    info!("Setting Windows desktop wallpaper registry keys");

    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key_colors = hkcu.open_subkey_with_flags("Control Panel\\Colors", KEY_WRITE)?;
    key_colors.set_value("Background", &"0 0 0")?;
    let key_desktop = hkcu.open_subkey_with_flags("Control Panel\\Desktop", KEY_WRITE)?;
    key_desktop.set_value("Wallpaper", &image_path.as_os_str())?;
    key_desktop.set_value("WallpaperStyle", &"6")?;
    key_desktop.set_value("TileWallpaper", &"0")?;

    // Also set wallpaper and fill color through user32 API
    info!("Setting Windows desktop wallpaper");

    use winapi::um::winnt::PVOID;
    use winapi::um::winuser::{COLOR_BACKGROUND, SPI_SETDESKWALLPAPER, SetSysColors, SystemParametersInfoW};

    // Background fill (black)
    unsafe {
        SetSysColors(1, [COLOR_BACKGROUND].as_ptr(), [0, 0, 0].as_ptr());
    }

    // Desktop wallpaper
    unsafe {
        let image_path = os_str_to_wchar(image_path.as_os_str());
        SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, image_path.as_ptr() as PVOID, 0);
    }

    Ok(())
}

fn os_str_to_wchar(oss: &std::ffi::OsStr) -> Vec<u16> {
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    // NUL-terminated unicode string
    oss.encode_wide().chain(once(0)).collect() 
}
