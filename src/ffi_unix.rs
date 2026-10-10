use log::info;
use std::path::Path;
use std::process::Command;
use url::Url;

use crate::error::AppErr;

pub fn set_wallpaper(image_path: &Path) -> Result<(), AppErr> {
    // TODO: Other OS Linux/OSX versions of try_set_XXX_wallpaper
    let result = try_set_gnome_wallpaper(image_path);
    match result {
        Ok(true) => {
            info!("Wallpaper updated");
            Ok(())
        }
        Ok(false) => Err(AppErr::from_generic(
            "set-wallpaper",
            "Setting the wallpaper is not supported on this platform",
        )),
        Err(err) => Err(AppErr::from_err("set-wallpaper", err)),
    }
}

fn try_set_gnome_wallpaper(image_path: &Path) -> Result<bool, AppErr> {
    // Test for gsettings - if not found, assume we're not using GNOME
    let gsettings_path = match Command::new("which").arg("gsettings").output() {
        // which fail, gsettings not found
        Ok(which) if !which.status.success() => return Ok(false),
        // which success, parse the path to gsettings
        Ok(which) => std::path::PathBuf::from(String::from_utf8(which.stdout)?.trim()),
        // which not found, or not executable
        Err(_) => return Ok(false),
    };

    let image_url = Url::from_file_path(image_path)
        .map_err(|_| AppErr::from_generic("Url", "Unable to convert path to file:// URL"))?;

    let _ = gsettings_set_wallpaper(
        &gsettings_path,
        GnomeBackgroundParameter::PictureUri,
        &image_url,
    );
    let _ = gsettings_set_wallpaper(
        &gsettings_path,
        GnomeBackgroundParameter::PictureUriDark,
        &image_url,
    );

    Ok(true)
}

enum GnomeBackgroundParameter {
    PictureUri,
    PictureUriDark,
}

fn gsettings_set_wallpaper(
    gsettings_path: &Path,
    parameter: GnomeBackgroundParameter,
    file_url: &Url,
) -> Result<(), AppErr> {
    // gsettings set org.gnome.desktop.background picture-uri "file:///path/to/your/image.jpg"
    let result = Command::new(gsettings_path)
        .args(["set", "org.gnome.desktop.background"])
        .arg(match parameter {
            GnomeBackgroundParameter::PictureUri => "picture-uri",
            GnomeBackgroundParameter::PictureUriDark => "picture-uri-dark",
        })
        .arg(file_url.as_str())
        .output()?;
    if result.status.success() {
        return Ok(());
    }
    let error = String::from_utf8_lossy(&result.stdout);
    return Err(AppErr::from_generic("gsettings", error));
}
