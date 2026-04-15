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
const CONTEXT_KEY: &str = "yImage.ContextMenu";
#[cfg(windows)]
const SUPPORTED_EXTS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".webp", ".bmp", ".gif", ".tif", ".tiff", ".avif",
];

/// Display labels for the cascading right-click menu. Passed in from the
/// caller so they can be localised to the user's current UI language.
#[derive(Clone)]
pub struct ContextMenuLabels {
    pub root: String,
    pub open: String,
    pub optimize: String,
    pub resize: String,
    pub convert: String,
    pub bg_remove: String,
    pub obj_remove: String,
}

impl Default for ContextMenuLabels {
    fn default() -> Self {
        Self {
            root: "yImage".into(),
            open: "Open with yImage".into(),
            optimize: "Optimize with yImage".into(),
            resize: "Resize with yImage".into(),
            convert: "Convert with yImage".into(),
            bg_remove: "Remove background (yImage)".into(),
            obj_remove: "Remove object (yImage)".into(),
        }
    }
}

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

/// Register a cascading "yImage" submenu on the right-click context menu for
/// every supported image extension.
///
/// We use the Shell `ExtendedSubCommandsKey` pattern (works entirely in HKCU —
/// no admin required): each extension points at a single shared command store
/// under `HKCU\Software\Classes\yImage.ContextMenu\shell\*`. Inside that store
/// we register verbs that each launch `yimage.exe --<action> "%1"`.
///
/// Entries appear on Windows 11 under the "Show more options" classic menu.
#[cfg(windows)]
pub fn register_context_menu(labels: &ContextMenuLabels) -> Result<()> {
    use std::env;
    use winreg::enums::*;
    use winreg::RegKey;

    let exe = env::current_exe()?.to_string_lossy().into_owned();
    let icon = format!("\"{exe}\",0");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. Per-extension cascading anchor: Shell\yImage pointing at the store.
    for ext in SUPPORTED_EXTS {
        let anchor_path =
            format!("Software\\Classes\\SystemFileAssociations\\{ext}\\Shell\\yImage");
        let (anchor, _) = hkcu.create_subkey(&anchor_path)?;
        anchor.set_value("MUIVerb", &labels.root.as_str())?;
        anchor.set_value("Icon", &icon)?;
        anchor.set_value("ExtendedSubCommandsKey", &CONTEXT_KEY)?;
    }

    // 2. Shared verb store: Software\Classes\yImage.ContextMenu\shell\<verb>
    //    Sibling verbs are sorted lexicographically in the menu, so we prefix
    //    with "01_".."06_" to keep a sensible order.
    let verbs: [(&str, &str, &str); 6] = [
        ("01_open", labels.open.as_str(), ""),
        ("02_optimize", labels.optimize.as_str(), "--optimize"),
        ("03_resize", labels.resize.as_str(), "--resize"),
        ("04_convert", labels.convert.as_str(), "--convert"),
        ("05_bg_remove", labels.bg_remove.as_str(), "--bg-remove"),
        ("06_obj_remove", labels.obj_remove.as_str(), "--obj-remove"),
    ];

    for (verb, label, flag) in verbs {
        let shell_path = format!("Software\\Classes\\{CONTEXT_KEY}\\shell\\{verb}");
        let (k, _) = hkcu.create_subkey(&shell_path)?;
        k.set_value("MUIVerb", &label)?;
        k.set_value("Icon", &icon)?;

        let command = if flag.is_empty() {
            format!("\"{exe}\" \"%1\"")
        } else {
            format!("\"{exe}\" {flag} \"%1\"")
        };
        let (cmd, _) = k.create_subkey("command")?;
        cmd.set_value("", &command)?;
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn register_context_menu(_labels: &ContextMenuLabels) -> Result<()> {
    Ok(())
}

/// Remove the cascading "yImage" submenu we installed with
/// [`register_context_menu`]. Leaves the ProgID / file associations alone.
#[cfg(windows)]
pub fn unregister_context_menu() -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for ext in SUPPORTED_EXTS {
        let anchor_path =
            format!("Software\\Classes\\SystemFileAssociations\\{ext}\\Shell\\yImage");
        // Best-effort delete — ignore "key not found" failures.
        let _ = hkcu.delete_subkey_all(&anchor_path);
    }
    let _ = hkcu.delete_subkey_all(format!("Software\\Classes\\{CONTEXT_KEY}"));
    Ok(())
}

#[cfg(not(windows))]
pub fn unregister_context_menu() -> Result<()> {
    Ok(())
}
