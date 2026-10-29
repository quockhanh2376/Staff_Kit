use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeIdentity {
    pub volume_serial_number: String,
    pub volume_name: String,
    pub file_system_name: String,
    pub drive_type: u32,
    pub is_removable_or_external: bool,
}

#[cfg(test)]
static MOCK_VOLUME_IDENTITIES: Mutex<Option<Vec<(PathBuf, VolumeIdentity)>>> = Mutex::new(None);
#[cfg(test)]
static MOCK_AVAILABLE_DISK_SPACE: Mutex<Option<Vec<(PathBuf, u64)>>> = Mutex::new(None);
#[cfg(test)]
static SIMULATE_USB_UNPLUG: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static SIMULATE_USB_CORRUPTION: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub fn reset_storage_safety_mocks() {
    *MOCK_VOLUME_IDENTITIES.lock().unwrap() = None;
    *MOCK_AVAILABLE_DISK_SPACE.lock().unwrap() = None;
    SIMULATE_USB_UNPLUG.store(false, Ordering::Release);
    SIMULATE_USB_CORRUPTION.store(false, Ordering::Release);
}

#[cfg(test)]
pub fn set_mock_volume_identity(path: &Path, identity: VolumeIdentity) {
    let mut guard = MOCK_VOLUME_IDENTITIES.lock().unwrap();
    let list = guard.get_or_insert_with(Vec::new);
    list.retain(|(p, _)| p != path);
    list.push((path.to_path_buf(), identity));
}

#[cfg(test)]
pub fn set_mock_available_disk_space(path: &Path, space: u64) {
    let mut guard = MOCK_AVAILABLE_DISK_SPACE.lock().unwrap();
    let list = guard.get_or_insert_with(Vec::new);
    list.retain(|(p, _)| p != path);
    list.push((path.to_path_buf(), space));
}

#[cfg(test)]
pub fn set_simulate_usb_unplug(unplug: bool) {
    SIMULATE_USB_UNPLUG.store(unplug, Ordering::Release);
}

#[cfg(test)]
pub fn is_simulate_usb_unplug() -> bool {
    SIMULATE_USB_UNPLUG.load(Ordering::Acquire)
}

#[cfg(test)]
pub fn set_simulate_usb_corruption(corrupt: bool) {
    SIMULATE_USB_CORRUPTION.store(corrupt, Ordering::Release);
}

#[cfg(test)]
pub fn is_simulate_usb_corruption() -> bool {
    SIMULATE_USB_CORRUPTION.load(Ordering::Acquire)
}

#[cfg(windows)]
fn extract_volume_root(path: &Path) -> Option<PathBuf> {
    let s = path.to_string_lossy();
    if s.len() >= 2 && s.as_bytes()[1] == b':' {
        let drive_prefix = &s[..2];
        return Some(PathBuf::from(format!("{}\\", drive_prefix)));
    }
    if s.starts_with("\\\\") {
        let parts: Vec<&str> = s[2..].split('\\').filter(|p| !p.is_empty()).collect();
        if parts.len() >= 2 {
            return Some(PathBuf::from(format!("\\\\{}\\{}\\", parts[0], parts[1])));
        }
    }
    None
}

#[allow(dead_code)]
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Retrieves volume identity (serial number, volume name, drive type) for a given path.
pub fn get_volume_identity(path: &Path) -> Result<VolumeIdentity, String> {
    #[cfg(test)]
    {
        if let Some(mocks) = &*MOCK_VOLUME_IDENTITIES.lock().unwrap() {
            let normalized_path = strip_verbatim_prefix(path);
            for (p, id) in mocks {
                let normalized_p = strip_verbatim_prefix(p);
                if normalized_path.starts_with(&normalized_p) || normalized_p.starts_with(&normalized_path) {
                    return Ok(id.clone());
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let root = extract_volume_root(path)
            .ok_or_else(|| format!("cannot determine volume root for path: {}", path.display()))?;
        let root_wide: Vec<u16> = OsStr::new(root.as_os_str())
            .encode_wide()
            .chain(Some(0))
            .collect();

        let mut vol_name_buf = [0u16; 261];
        let mut fs_name_buf = [0u16; 261];
        let mut serial_num: u32 = 0;
        let mut max_comp_len: u32 = 0;
        let mut flags: u32 = 0;

        let res = unsafe {
            windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW(
                root_wide.as_ptr(),
                vol_name_buf.as_mut_ptr(),
                vol_name_buf.len() as u32,
                &mut serial_num,
                &mut max_comp_len,
                &mut flags,
                fs_name_buf.as_mut_ptr(),
                fs_name_buf.len() as u32,
            )
        };

        if res == 0 {
            return Err(format!(
                "failed to query volume information for {}: OS error",
                root.display()
            ));
        }

        let drive_type = unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root_wide.as_ptr())
        };

        let vol_len = vol_name_buf.iter().position(|&c| c == 0).unwrap_or(0);
        let vol_name = String::from_utf16_lossy(&vol_name_buf[..vol_len]);

        let fs_len = fs_name_buf.iter().position(|&c| c == 0).unwrap_or(0);
        let fs_name = String::from_utf16_lossy(&fs_name_buf[..fs_len]);

        // DRIVE_REMOVABLE = 2
        // DRIVE_FIXED = 3 (external USB drives may report fixed, so serial comparison with local drives is key)
        let is_removable = drive_type == 2;

        Ok(VolumeIdentity {
            volume_serial_number: format!("{:08X}", serial_num),
            volume_name: vol_name,
            file_system_name: fs_name,
            drive_type,
            is_removable_or_external: is_removable,
        })
    }

    #[cfg(not(windows))]
    {
        Ok(VolumeIdentity {
            volume_serial_number: "SIMULATED_SERIAL_001".to_string(),
            volume_name: "StaffKitUSB".to_string(),
            file_system_name: "NTFS".to_string(),
            drive_type: 2,
            is_removable_or_external: true,
        })
    }
}

/// Retrieves available free disk space in bytes for a given directory.
pub fn get_available_disk_space(path: &Path) -> Result<u64, String> {
    #[cfg(test)]
    {
        if let Some(mocks) = &*MOCK_AVAILABLE_DISK_SPACE.lock().unwrap() {
            let normalized_path = strip_verbatim_prefix(path);
            for (p, space) in mocks {
                let normalized_p = strip_verbatim_prefix(p);
                if normalized_path.starts_with(&normalized_p) || normalized_p.starts_with(&normalized_path) {
                    return Ok(*space);
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let wide: Vec<u16> = OsStr::new(path.as_os_str())
            .encode_wide()
            .chain(Some(0))
            .collect();

        let mut free_bytes_to_caller: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        let res = unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes_to_caller,
                &mut total_bytes,
                &mut total_free_bytes,
            )
        };

        if res == 0 {
            return Err(format!(
                "failed to query available disk space for {}: OS error",
                path.display()
            ));
        }

        Ok(free_bytes_to_caller)
    }

    #[cfg(not(windows))]
    {
        Ok(10 * 1024 * 1024 * 1024) // 10 GB default for non-windows
    }
}

pub fn is_network_unc_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.starts_with(r"\\?\UNC\") {
        return true;
    }
    if s.starts_with(r"\\?\") {
        return false;
    }
    if s.starts_with(r"\\") || s.starts_with("//") {
        return true;
    }
    false
}

/// Validates whether a candidate USB destination is safe, valid, and separate
/// from the primary IT machine's local DB storage.
pub fn validate_usb_destination(
    usb_candidate_path: &Path,
    db_path: &Path,
    local_backup_dir: &Path,
) -> Result<VolumeIdentity, String> {
    if is_network_unc_path(usb_candidate_path) {
        return Err("network UNC storage paths cannot be registered as USB independent destination".to_string());
    }

    if !usb_candidate_path.exists() {
        fs::create_dir_all(usb_candidate_path)
            .map_err(|err| format!("failed to access or create USB directory: {err}"))?;
    }

    let usb_canonical = fs::canonicalize(usb_candidate_path)
        .map_err(|err| format!("cannot canonicalize USB destination path: {err}"))?;

    if is_network_unc_path(&usb_canonical) {
        return Err("network UNC storage paths cannot be registered as USB independent destination".to_string());
    }

    // 1. Check if same folder or child/parent of DB directory
    let db_parent = db_path.parent().unwrap_or(db_path);
    if let Ok(db_parent_canon) = fs::canonicalize(db_parent) {
        if usb_canonical == db_parent_canon
            || usb_canonical.starts_with(&db_parent_canon)
            || db_parent_canon.starts_with(&usb_canonical)
        {
            return Err("USB destination cannot be inside or share the database directory".to_string());
        }
    }

    // 2. Check if same folder or child of local backup directory
    if let Ok(local_backup_canon) = fs::canonicalize(local_backup_dir) {
        if usb_canonical == local_backup_canon
            || usb_canonical.starts_with(&local_backup_canon)
            || local_backup_canon.starts_with(&usb_canonical)
        {
            return Err("USB destination cannot be inside or share the local backup directory".to_string());
        }
    }

    // 3. Query volume identity of USB candidate, DB path, and local backup directory
    let usb_identity = get_volume_identity(&usb_canonical)?;
    let db_identity = get_volume_identity(db_path)?;
    let local_backup_identity = get_volume_identity(local_backup_dir)?;

    // Reject if network drive (DRIVE_REMOTE = 4)
    if usb_identity.drive_type == 4 {
        return Err("network drive cannot be registered as an independent USB storage destination".to_string());
    }

    // Reject if on the same physical volume/disk as the local DB
    if usb_identity.volume_serial_number == db_identity.volume_serial_number {
        return Err(format!(
            "USB destination must be on an independent drive, but shares volume serial number ({}) with the database storage",
            usb_identity.volume_serial_number
        ));
    }

    // Reject if on the same physical volume/disk as the local backup directory
    if usb_identity.volume_serial_number == local_backup_identity.volume_serial_number {
        return Err(format!(
            "USB destination must be on an independent drive, but shares volume serial number ({}) with the local backup directory",
            usb_identity.volume_serial_number
        ));
    }

    // 4. Test writability by writing and deleting a temporary probe file
    let probe_file = usb_canonical.join(".staff_kit_probe.tmp");
    fs::write(&probe_file, b"staff_kit_write_test")
        .map_err(|err| format!("USB destination is not writable: {err}"))?;
    let _ = fs::remove_file(&probe_file);

    Ok(usb_identity)
}
