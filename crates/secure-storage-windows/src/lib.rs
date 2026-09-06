use thiserror::Error;
use tracing::{error, info};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Credential not found: {0}")]
    NotFound(String),

    #[error("Windows Credential API error code: {0}")]
    Win32Error(u32),

    #[error("Invalid UTF-8 string in stored credential")]
    InvalidUtf8,

    #[error("Platform unsupported")]
    UnsupportedPlatform,
}

pub const DEFAULT_TARGET_NAME: &str = "Snipe/AI/ApiKey";

pub struct CredentialStorage;

impl CredentialStorage {
    /// Mask an API key for UI display (e.g. "sk-...ab12")
    pub fn mask_key(key: &str) -> String {
        if key.is_empty() {
            return "未配置".to_string();
        }
        if key.len() <= 8 {
            return "********".to_string();
        }
        let prefix = &key[..3.min(key.len())];
        let suffix = &key[key.len().saturating_sub(4)..];
        format!("{}...{}", prefix, suffix)
    }

    #[cfg(windows)]
    pub fn store_secret(target_name: &str, secret: &str) -> Result<(), StorageError> {
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        };

        let target_wide: Vec<u16> = target_name.encode_utf16().chain(std::iter::once(0)).collect();
        let user_wide: Vec<u16> = "SnipeUser".encode_utf16().chain(std::iter::once(0)).collect();
        let secret_bytes = secret.as_bytes();

        let mut cred = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: PCWSTR(target_wide.as_ptr()),
            Comment: PCWSTR::null(),
            LastWritten: windows::Win32::Foundation::FILETIME::default(),
            CredentialBlobSize: secret_bytes.len() as u32,
            CredentialBlob: secret_bytes.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: PCWSTR::null(),
            UserName: PCWSTR(user_wide.as_ptr()),
        };

        unsafe {
            CredWriteW(&mut cred, 0).map_err(|e| {
                error!("Failed to write credential for {}: {:?}", target_name, e);
                StorageError::Win32Error(e.code().0 as u32)
            })?;
        }

        info!("Stored credential securely in Credential Manager: {}", target_name);
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn store_secret(_target_name: &str, _secret: &str) -> Result<(), StorageError> {
        Err(StorageError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    pub fn read_secret(target_name: &str) -> Result<String, StorageError> {
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{
            CredFree, CredReadW, PCREDENTIALW, CRED_TYPE_GENERIC,
        };

        let target_wide: Vec<u16> = target_name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut p_cred: *mut windows::Win32::Security::Credentials::CREDENTIALW = std::ptr::null_mut();

        unsafe {
            let res = CredReadW(
                PCWSTR(target_wide.as_ptr()),
                CRED_TYPE_GENERIC,
                0,
                &mut p_cred as *mut PCREDENTIALW as *mut _,
            );

            if let Err(e) = res {
                return Err(StorageError::NotFound(e.to_string()));
            }

            if p_cred.is_null() {
                return Err(StorageError::NotFound(target_name.to_string()));
            }

            let cred = &*p_cred;
            let blob_slice = std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
            let secret = String::from_utf8(blob_slice.to_vec()).map_err(|_| StorageError::InvalidUtf8)?;

            CredFree(p_cred as *mut _);
            Ok(secret)
        }
    }

    #[cfg(not(windows))]
    pub fn read_secret(_target_name: &str) -> Result<String, StorageError> {
        Err(StorageError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    pub fn delete_secret(target_name: &str) -> Result<(), StorageError> {
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

        let target_wide: Vec<u16> = target_name.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0).map_err(|e| {
                StorageError::Win32Error(e.code().0 as u32)
            })?;
        }
        info!("Deleted credential from Credential Manager: {}", target_name);
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn delete_secret(_target_name: &str) -> Result<(), StorageError> {
        Err(StorageError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key() {
        assert_eq!(CredentialStorage::mask_key(""), "未配置");
        assert_eq!(CredentialStorage::mask_key("sk-1234"), "********");
        assert_eq!(CredentialStorage::mask_key("sk-proj-12345678abcdef"), "sk-...cdef");
    }
}
