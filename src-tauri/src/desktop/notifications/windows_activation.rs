//! A running desktop COM server for Notification Center activation. There is no
//! LocalServer32 command registration: expired notifications cannot launch a new
//! app process after Quit. Targets are meaningful only in this process/session.
use super::{super::windows_state::Targets, *};
use ::windows::{
    Win32::{
        Foundation::{
            CLASS_E_NOAGGREGATION, CloseHandle, E_POINTER, ERROR_ALREADY_EXISTS, GetLastError,
            HANDLE,
        },
        System::{
            Com::{
                CLSCTX_LOCAL_SERVER, CoRegisterClassObject, CoRevokeClassObject, IClassFactory,
                IClassFactory_Impl, REGCLS_MULTIPLEUSE,
            },
            Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
                RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
            },
            Threading::CreateMutexW,
        },
        UI::Notifications::{
            INotificationActivationCallback, INotificationActivationCallback_Impl,
            NOTIFICATION_USER_INPUT_DATA,
        },
    },
    core::{BOOL, GUID, IUnknown, Interface, PCWSTR, Ref, implement, w},
};
use std::ffi::c_void;

const CLASS_ID: GUID = GUID::from_u128(0x39d7de04_8af7_49ec_97ce_44aeec0f73b8);
const CLASS_TEXT: &str = "{39D7DE04-8AF7-49EC-97CE-44AEEC0F73B8}";

#[implement(INotificationActivationCallback)]
struct Activator {
    shared: Arc<Shared>,
    targets: Arc<Mutex<Targets>>,
}
impl INotificationActivationCallback_Impl for Activator_Impl {
    fn Activate(
        &self,
        app_id: &PCWSTR,
        arguments: &PCWSTR,
        _: *const NOTIFICATION_USER_INPUT_DATA,
        _: u32,
    ) -> ::windows::core::Result<()> {
        // SAFETY: COM marshals these NUL-terminated input strings for the call.
        // Bound decoding; no input is treated as a URL, path or command.
        let values = unsafe { (bounded_string(*app_id, 64), bounded_string(*arguments, 32)) };
        if let (Some(app_id), Some(id)) = values {
            activate(&self.shared, &self.targets, &app_id, &id);
        }
        Ok(())
    }
}
unsafe fn bounded_string(value: PCWSTR, max: usize) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let mut units = Vec::new();
    for index in 0..=max {
        // SAFETY: caller supplies a valid NUL-terminated COM string; stop at NUL.
        let unit = unsafe { *value.0.add(index) };
        if unit == 0 {
            return String::from_utf16(&units).ok();
        }
        if index == max {
            return None;
        }
        units.push(unit);
    }
    None
}
pub(super) fn activate(shared: &Shared, targets: &Mutex<Targets>, app_id: &str, id: &str) {
    let event = targets
        .lock()
        .expect("notification targets poisoned")
        .activate(app_id, id, shared.stop.is_cancelled());
    if let Some(event) = event {
        shared.activate(&event);
    }
}
#[implement(IClassFactory)]
struct Factory {
    shared: Arc<Shared>,
    targets: Arc<Mutex<Targets>>,
}
impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<'_, IUnknown>,
        iid: *const GUID,
        result: *mut *mut c_void,
    ) -> ::windows::core::Result<()> {
        if iid.is_null() || result.is_null() {
            return Err(E_POINTER.into());
        }
        // SAFETY: COM provides writable output storage and a valid interface ID.
        unsafe {
            *result = std::ptr::null_mut();
        }
        if outer.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let callback: INotificationActivationCallback = Activator {
            shared: self.shared.clone(),
            targets: self.targets.clone(),
        }
        .into();
        // SAFETY: QueryInterface transfers its own reference to the caller.
        unsafe { callback.query(iid, result).ok() }
    }
    fn LockServer(&self, _: BOOL) -> ::windows::core::Result<()> {
        Ok(())
    }
}
pub(super) struct Registration {
    cookie: u32,
    key: HKEY,
    _owner: ActivationOwner,
}
// A second app can still open with unavailable credential storage. It must not
// replace the first instance's AUMID route or clear its Notification Center.
struct ActivationOwner(HANDLE);
impl ActivationOwner {
    fn claim(name: PCWSTR) -> ::windows::core::Result<Self> {
        // SAFETY: fixed session-local object name; the handle keeps the object
        // alive without owning the mutex or depending on thread exit semantics.
        let handle = unsafe { CreateMutexW(None, false, name)? };
        let owner = Self(handle);
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return Err(::windows::core::Error::from_hresult(
                ERROR_ALREADY_EXISTS.to_hresult(),
            ));
        }
        Ok(owner)
    }
}
impl Drop for ActivationOwner {
    fn drop(&mut self) {
        // SAFETY: close exactly the owned handle returned by CreateMutexW.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
impl Registration {
    pub fn new(shared: Arc<Shared>, targets: Arc<Mutex<Targets>>) -> ::windows::core::Result<Self> {
        let owner = ActivationOwner::claim(w!("Local\\StreamGuiRS.NotificationActivation"))?;
        let factory: IClassFactory = Factory { shared, targets }.into();
        // SAFETY: called on the notification worker's initialized MTA; COM owns
        // a factory reference until CoRevokeClassObject on that same worker.
        let cookie = unsafe {
            CoRegisterClassObject(&CLASS_ID, &factory, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE)?
        };
        let mut key = HKEY::default();
        // Register the running class under the existing application identity,
        // following the Windows desktop notification registry activation model.
        // SAFETY: fixed app-owned path and valid output handle storage.
        let opened = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                w!("Software\\Classes\\AppUserModelId\\io.github.stream-gui-rs"),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
            .ok()
        };
        if let Err(error) = opened {
            unsafe {
                let _ = CoRevokeClassObject(cookie);
            }
            return Err(error);
        }
        let owner = Self {
            cookie,
            key,
            _owner: owner,
        };
        owner.set(w!("DisplayName"), "Stream GUI RS")?;
        owner.set(w!("CustomActivator"), CLASS_TEXT)?;
        Ok(owner)
    }
    fn set(&self, name: PCWSTR, value: &str) -> ::windows::core::Result<()> {
        let bytes: Vec<u8> = value
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        // SAFETY: owned key and NUL-terminated UTF-16 bytes live through the call.
        unsafe { RegSetValueExW(self.key, name, None, REG_SZ, Some(&bytes)).ok() }
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        // The notification identity guard excludes a competing registration.
        // Keep the OS display identity/notification preferences, retire only this
        // running activation route and its COM factory. Never register execution.
        unsafe {
            let _ = RegDeleteValueW(self.key, w!("CustomActivator"));
            let _ = RegCloseKey(self.key);
            let _ = CoRevokeClassObject(self.cookie);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_owner_excludes_competitors_and_releases_on_drop() {
        // Isolated kernel object: never contend with the running app's identity.
        let name = ::windows::core::HSTRING::from(format!(
            "Local\\StreamGuiRSTest-{}",
            uuid::Uuid::new_v4()
        ));
        let owner = ActivationOwner::claim(PCWSTR(name.as_ptr())).unwrap();
        assert!(ActivationOwner::claim(PCWSTR(name.as_ptr())).is_err());
        drop(owner);
        assert!(ActivationOwner::claim(PCWSTR(name.as_ptr())).is_ok());
    }
    #[test]
    fn activation_payload_decoding_is_bounded_and_rejects_invalid_utf16() {
        let valid = ::windows::core::HSTRING::from("12345678901234567890123456789012");
        let overlong = ::windows::core::HSTRING::from("123456789012345678901234567890123");
        let invalid = [0xd800, 0];
        // SAFETY: all inputs are valid, live NUL-terminated buffers (including
        // the intentionally malformed UTF-16 contents).
        unsafe {
            assert!(bounded_string(PCWSTR(valid.as_ptr()), 32).is_some());
            assert!(bounded_string(PCWSTR(overlong.as_ptr()), 32).is_none());
            assert!(bounded_string(PCWSTR(invalid.as_ptr()), 32).is_none());
            assert!(bounded_string(PCWSTR::null(), 32).is_none());
        }
    }
}
