// Windows file-association / default-program registration.
//
// Writes a per-user ProgID and open verb under HKCU so the app can be set as
// the default opener for common image extensions without requiring an
// elevated (admin) process. The Inno Setup installer additionally writes
// `HKCU\Software\RegisteredApplications` so yImage appears in the Windows 10/11
// "Default apps" UI.

#![cfg_attr(not(windows), allow(dead_code))]

use anyhow::Result;

#[cfg(windows)]
const PROG_ID: &str = "yImage.Image.1";
#[cfg(windows)]
const SUPPORTED_EXTS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".webp", ".bmp", ".gif", ".tif", ".tiff", ".avif",
];

#[cfg(windows)]
pub fn register_file_associations() -> Result<()> {
    use std::env;
    use winreg::enums::*;
    use winreg::RegKey;

    let exe = env::current_exe()?.to_string_lossy().into_owned();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. ProgID: HKCU\Software\Classes\yImage.Image.1
    let (prog, _) = hkcu.create_subkey(format!("Software\\Classes\\{PROG_ID}"))?;
    prog.set_value("", &"yImage")?;
    let (icon, _) = prog.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{exe}\",0"))?;
    let (cmd, _) = prog.create_subkey("shell\\open\\command")?;
    cmd.set_value("", &format!("\"{exe}\" \"%1\""))?;

    // 2. Per-extension OpenWithProgids entry so users can "Open with" yImage.
    for ext in SUPPORTED_EXTS {
        let path = format!("Software\\Classes\\{ext}\\OpenWithProgids");
        let (k, _) = hkcu.create_subkey(&path)?;
        k.set_value(PROG_ID, &"")?;
    }

    // 3. RegisteredApplications so Windows' "Default apps" settings lists us.
    let (caps, _) = hkcu.create_subkey("Software\\yImage\\Capabilities")?;
    caps.set_value("ApplicationName", &"yImage")?;
    caps.set_value("ApplicationDescription", &"Fast image viewer and editor")?;
    let (file_assocs, _) = caps.create_subkey("FileAssociations")?;
    for ext in SUPPORTED_EXTS {
        file_assocs.set_value(ext, &PROG_ID)?;
    }
    let (reg_apps, _) = hkcu.create_subkey("Software\\RegisteredApplications")?;
    reg_apps.set_value("yImage", &"Software\\yImage\\Capabilities")?;

    Ok(())
}

#[cfg(not(windows))]
pub fn register_file_associations() -> Result<()> {
    Ok(())
}
