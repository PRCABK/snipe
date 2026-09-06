use thiserror::Error;
#[cfg(windows)]
use tracing::{error, info};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Credential not found: {0}")]
    NotFound(String),

    #[error("Windows Credential API error HRESULT: {0:#010x}")]
    WindowsError(i32),

    #[error("Credential target name must not be empty or contain NUL")]
    InvalidTargetName,

    #[error("Credential is too large: {actual} bytes (maximum {maximum})")]
    SecretTooLarge { actual: usize, maximum: usize },

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
        let chars: Vec<char> = key.chars().collect();
        if chars.is_empty() {
            return "未配置".to_string();
        }
        if chars.len() <= 8 {
            return "********".to_string();
        }

        let prefix: String = chars.iter().take(3).collect();
        let suffix: String = chars.iter().skip(chars.len() - 4).collect();
        format!("{prefix}...{suffix}")
    }

    #[cfg(windows)]
    pub fn store_secret(target_name: &str, secret: &str) -> Result<(), StorageError> {
        use windows::core::PWSTR;
        use windows::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        };

        const MAX_CREDENTIAL_BLOB_SIZE: usize = 5 * 512;
        if target_name.is_empty() || target_name.contains('\0') {
            return Err(StorageError::InvalidTargetName);
        }
        if secret.len() > MAX_CREDENTIAL_BLOB_SIZE {
            return Err(StorageError::SecretTooLarge {
                actual: secret.len(),
                maximum: MAX_CREDENTIAL_BLOB_SIZE,
            });
        }

        let mut target_wide: Vec<u16> = target_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut user_wide: Vec<u16> = "SnipeUser"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let secret_bytes = secret.as_bytes();

        let cred = CREDENTIALW {
            Flags: CRED_FLAGS(0),
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target_wide.as_mut_ptr()),
            Comment: PWSTR::null(),
            LastWritten: windows::Win32::Foundation::FILETIME::default(),
            CredentialBlobSize: secret_bytes.len() as u32,
            CredentialBlob: secret_bytes.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: PWSTR::null(),
            UserName: PWSTR(user_wide.as_mut_ptr()),
        };

        unsafe {
            CredWriteW(&cred, 0).map_err(|err| {
                error!("Failed to write credential for {target_name}: {err}");
                StorageError::WindowsError(err.code().0)
            })?;
        }

        info!(
            "Stored credential securely in Credential Manager: {}",
            target_name
        );
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn store_secret(_target_name: &str, _secret: &str) -> Result<(), StorageError> {
        Err(StorageError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    pub fn read_secret(target_name: &str) -> Result<String, StorageError> {
        use windows::core::{HRESULT, PCWSTR};
        use windows::Win32::Foundation::ERROR_NOT_FOUND;

        if target_name.is_empty() || target_name.contains('\0') {
            return Err(StorageError::InvalidTargetName);
        }
        use windows::Win32::Security::Credentials::{
            CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };

        let target_wide: Vec<u16> = target_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut credential: *mut CREDENTIALW = std::ptr::null_mut();

        unsafe {
            if let Err(err) = CredReadW(
                PCWSTR(target_wide.as_ptr()),
                CRED_TYPE_GENERIC,
                0,
                &mut credential,
            ) {
                if err.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0) {
                    return Err(StorageError::NotFound(target_name.to_string()));
                }
                return Err(StorageError::WindowsError(err.code().0));
            }

            if credential.is_null() {
                return Err(StorageError::NotFound(target_name.to_string()));
            }

            let cred = &*credential;
            let blob = if cred.CredentialBlobSize == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize)
                    .to_vec()
            };
            CredFree(credential.cast::<std::ffi::c_void>() as *const std::ffi::c_void);

            String::from_utf8(blob).map_err(|_| StorageError::InvalidUtf8)
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

        if target_name.is_empty() || target_name.contains('\0') {
            return Err(StorageError::InvalidTargetName);
        }

        let target_wide: Vec<u16> = target_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0).map_err(|err| {
                if err.code()
                    == windows::core::HRESULT::from_win32(
                        windows::Win32::Foundation::ERROR_NOT_FOUND.0,
                    )
                {
                    StorageError::NotFound(target_name.to_string())
                } else {
                    StorageError::WindowsError(err.code().0)
                }
            })?;
        }
        info!(
            "Deleted credential from Credential Manager: {}",
            target_name
        );
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
        assert_eq!(
            CredentialStorage::mask_key("sk-proj-12345678abcdef"),
            "sk-...cdef"
        );
        assert_eq!(
            CredentialStorage::mask_key("密钥前缀-123456"),
            "密钥前...3456"
        );
    }

    #[cfg(windows)]
    #[test]
    fn rejects_invalid_target_name_and_oversized_secrets() {
        assert!(matches!(
            CredentialStorage::store_secret("", "secret"),
            Err(StorageError::InvalidTargetName)
        ));
        assert!(matches!(
            CredentialStorage::store_secret("bad\0target", "secret"),
            Err(StorageError::InvalidTargetName)
        ));
        assert!(matches!(
            CredentialStorage::store_secret("Snipe/Test", &"x".repeat(2561)),
            Err(StorageError::SecretTooLarge { .. })
        ));
    }
}
