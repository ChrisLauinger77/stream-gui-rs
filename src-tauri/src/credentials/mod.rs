use crate::domain::Result;
use zeroize::Zeroizing;

/// Deliberately not Debug, Serialize or TS. These values never cross IPC.
#[derive(Clone)]
pub struct Credentials {
    pub access_token: Zeroizing<String>,
    pub refresh_token: Zeroizing<String>,
}

/// Phase 0 stores credentials only for the current Rust process. A future OS
/// keychain adapter can implement this contract without changing React or OAuth.
pub trait CredentialStore: Send {
    fn load(&self) -> Result<Option<Credentials>>;
    fn save(&mut self, credentials: Credentials) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
    fn name(&self) -> &'static str;
}

#[derive(Default)]
pub struct MemoryCredentialStore(Option<Credentials>);

impl CredentialStore for MemoryCredentialStore {
    fn load(&self) -> Result<Option<Credentials>> {
        Ok(self.0.clone())
    }
    fn save(&mut self, credentials: Credentials) -> Result<()> {
        self.0 = Some(credentials);
        Ok(())
    }
    fn clear(&mut self) -> Result<()> {
        self.0 = None;
        Ok(())
    }
    fn name(&self) -> &'static str {
        "memory (this app session only)"
    }
}
