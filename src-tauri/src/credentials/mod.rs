use crate::domain::Result;
use zeroize::Zeroizing;

/// Deliberately not Debug, Serialize or TS. These values never cross IPC.
#[derive(Clone)]
pub struct Credentials {
    pub access_token: Zeroizing<String>,
    pub refresh_token: Zeroizing<String>,
}

/// Rust-only storage contract. Desktop uses the OS credential store; deterministic
/// tests use memory without touching the user's keychain.
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

#[cfg(any(target_os = "macos", target_os = "linux", windows))]
mod platform;
#[cfg(any(target_os = "macos", target_os = "linux", windows))]
pub use platform::PlatformCredentialStore;

pub struct UnavailableCredentialStore(pub crate::domain::AppError);
impl CredentialStore for UnavailableCredentialStore {
    fn load(&self) -> Result<Option<Credentials>> {
        Err(self.0.clone())
    }
    fn save(&mut self, _: Credentials) -> Result<()> {
        Err(self.0.clone())
    }
    fn clear(&mut self) -> Result<()> {
        Err(self.0.clone())
    }
    fn name(&self) -> &'static str {
        "unavailable (no insecure fallback)"
    }
}

/// OS keychain calls may prompt or use IPC. Keep them off Tokio workers and
/// serialize all access to an entry. Only the auth service owns this vault.
pub struct CredentialVault(std::sync::Arc<std::sync::Mutex<Box<dyn CredentialStore>>>);
impl CredentialVault {
    pub fn new(store: Box<dyn CredentialStore>) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(store)))
    }
    async fn run<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut dyn CredentialStore) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let store = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let mut store = store.lock().map_err(|_| vault_error())?;
            operation(store.as_mut())
        })
        .await
        .map_err(|_| vault_error())?
    }
    pub async fn load(&self) -> Result<Option<Credentials>> {
        self.run(|store| store.load()).await
    }
    pub async fn save(&self, credentials: Credentials) -> Result<()> {
        self.run(move |store| store.save(credentials)).await
    }
    pub async fn clear(&self) -> Result<()> {
        self.run(|store| store.clear()).await
    }
}
fn vault_error() -> crate::domain::AppError {
    crate::domain::AppError::new(
        crate::domain::ErrorCode::CredentialStore,
        "The secure credential operation could not complete.",
    )
}
