use super::{CredentialStore, Credentials};
use crate::domain::{AppError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};
use zeroize::Zeroizing;

const SERVICE: &str = "io.github.stream-gui-rs.oauth";

pub struct PlatformCredentialStore {
    entry: keyring::Entry,
    client_id: String,
    // Only one process may rotate this application's stored credentials.
    _lease: File,
}

impl PlatformCredentialStore {
    #[cfg(test)]
    pub(crate) fn with_test_credential(credential: Box<keyring::credential::Credential>) -> Self {
        Self {
            entry: keyring::Entry::new_with_credential(credential),
            client_id: "client".into(),
            _lease: tempfile::tempfile().unwrap(),
        }
    }

    pub fn open(directory: &Path, client_id: &str) -> Result<Self> {
        std::fs::create_dir_all(directory).map_err(|_| storage_error())?;
        let lease = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("authentication.lock"))
            .map_err(|_| storage_error())?;
        fs2::FileExt::try_lock_exclusive(&lease).map_err(|_| AppError::new(
            ErrorCode::CredentialStore, "Authentication is already owned by another application instance, or its lock is unavailable."))?;
        let entry = keyring::Entry::new(SERVICE, client_id).map_err(|_| storage_error())?;
        Ok(Self {
            entry,
            client_id: client_id.into(),
            _lease: lease,
        })
    }
}

// This private storage format is never an IPC DTO. Both tokens are replaced in
// one secure-store entry; normal settings contain no credential material.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCredentials {
    version: u32,
    client_id: String,
    access_token: Zeroizing<String>,
    refresh_token: Zeroizing<String>,
}

fn encode(credentials: Credentials, client_id: &str) -> Result<Zeroizing<Vec<u8>>> {
    let stored = StoredCredentials {
        version: 1,
        client_id: client_id.into(),
        access_token: credentials.access_token,
        refresh_token: credentials.refresh_token,
    };
    serde_json::to_vec(&stored)
        .map(Zeroizing::new)
        .map_err(|_| storage_error())
}
fn decode(bytes: &[u8], client_id: &str) -> Result<Credentials> {
    if bytes.len() > 16 * 1024 {
        return Err(storage_error());
    }
    let stored: StoredCredentials = serde_json::from_slice(bytes).map_err(|_| storage_error())?;
    if stored.version != 1
        || stored.client_id != client_id
        || stored.access_token.is_empty()
        || stored.refresh_token.is_empty()
    {
        return Err(storage_error());
    }
    Ok(Credentials {
        access_token: stored.access_token,
        refresh_token: stored.refresh_token,
    })
}

impl CredentialStore for PlatformCredentialStore {
    fn load(&self) -> Result<Option<Credentials>> {
        match self.entry.get_secret() {
            Ok(bytes) => decode(&Zeroizing::new(bytes), &self.client_id).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(storage_error()),
        }
    }
    fn save(&mut self, credentials: Credentials) -> Result<()> {
        self.entry
            .set_secret(&encode(credentials, &self.client_id)?)
            .map_err(|_| storage_error())
    }
    fn clear(&mut self) -> Result<()> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(_) => return Err(storage_error()),
        }
        // The macOS dependency can discard the native deletion error. Only an
        // explicit NoEntry establishes absence; unreadable is not deleted.
        match self.entry.get_secret() {
            Err(keyring::Error::NoEntry) => Ok(()),
            Ok(bytes) => {
                drop(Zeroizing::new(bytes));
                Err(storage_error())
            }
            Err(_) => Err(storage_error()),
        }
    }
    fn name(&self) -> &'static str {
        if cfg!(target_os = "macos") {
            "macOS Keychain"
        } else if cfg!(windows) {
            "Windows Credential Manager"
        } else {
            "Linux Secret Service"
        }
    }
}

pub(super) fn storage_error() -> AppError {
    AppError::new(
        ErrorCode::CredentialStore,
        "Secure credential storage is unavailable, locked, or invalid. Unlock the OS credential store and retry; no plaintext fallback is used.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_record_roundtrip_rejects_other_clients_versions_and_malformed_data() {
        let credentials = Credentials {
            access_token: Zeroizing::new("test-access".into()),
            refresh_token: Zeroizing::new("test-refresh".into()),
        };
        let encoded = encode(credentials, "client").unwrap();
        let restored = decode(&encoded, "client").unwrap();
        assert_eq!(&*restored.access_token, "test-access");
        assert_eq!(&*restored.refresh_token, "test-refresh");
        assert!(decode(&encoded, "different").is_err());
        for data in [b"test-secret".as_slice(), br#"{"version":2,"client_id":"client","access_token":"secret","refresh_token":"secret"}"#] {
            let error = decode(data, "client").err().unwrap();
            assert!(!error.to_string().contains("secret"));
        }
    }
    #[test]
    fn platform_adapter_rotates_and_deletes_one_entry_without_os_access() {
        let entry =
            keyring::Entry::new_with_credential(Box::new(keyring::mock::MockCredential::default()));
        let mut store = PlatformCredentialStore {
            entry,
            client_id: "client".into(),
            _lease: tempfile::tempfile().unwrap(),
        };
        assert!(store.load().unwrap().is_none());
        for suffix in ["first", "rotated"] {
            store
                .save(Credentials {
                    access_token: Zeroizing::new(format!("access-{suffix}")),
                    refresh_token: Zeroizing::new(format!("refresh-{suffix}")),
                })
                .unwrap();
            assert_eq!(
                store.load().unwrap().unwrap().refresh_token.as_str(),
                format!("refresh-{suffix}")
            );
        }
        store.clear().unwrap();
        store.clear().unwrap();
        assert!(store.load().unwrap().is_none());
    }
}
