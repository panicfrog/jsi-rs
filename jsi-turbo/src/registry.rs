//! Module registry: linkme-distributed auto-registration plus a manual
//! fallback registry.
//!
//! The `#[RustTurboModule]` macro contributes one [`ModuleDescriptor`] to the
//! [`TURBO_MODULES`] distributed slice per module. On Android (cdylib) and in
//! Rust binaries/tests all entries are always linked in. On iOS (staticlib),
//! module crates also export a `<name>_force` anchor function that the app's
//! aggregate library must reference so the crate's objects (and slice
//! entries) survive archive-level dead-code elimination — verified by the
//! Phase 0 linkme spike.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::holder::ModuleHolder;

/// Describes one registered module.
#[derive(Clone, Copy)]
pub struct ModuleDescriptor {
    /// The name JavaScript uses in `RustTurbo.get(name)`.
    pub name: &'static str,
    /// Constructs the module instance.
    pub ctor: fn() -> anyhow::Result<ModuleHolder>,
}

#[doc(hidden)]
#[linkme::distributed_slice]
pub static TURBO_MODULES: [ModuleDescriptor] = [..];

static MANUAL: Mutex<Option<HashMap<&'static str, ModuleDescriptor>>> = Mutex::new(None);

/// Manually registers a module (the fallback for setups where the linkme
/// slice cannot be used, e.g. runtime-defined modules).
pub fn register(descriptor: ModuleDescriptor) {
    let mut guard = MANUAL.lock().unwrap();
    guard.get_or_insert_with(HashMap::new).insert(descriptor.name, descriptor);
}

/// Looks up a module by name (distributed slice first, then manual registry).
pub fn lookup(name: &str) -> Option<ModuleDescriptor> {
    for entry in TURBO_MODULES.iter() {
        if entry.name == name {
            return Some(*entry);
        }
    }
    MANUAL
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|m| m.get(name).copied())
}

/// All registered module names (slice + manual), for diagnostics.
pub fn registered_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = TURBO_MODULES.iter().map(|e| e.name).collect();
    if let Some(m) = MANUAL.lock().unwrap().as_ref() {
        names.extend(m.keys().copied());
    }
    names.sort_unstable();
    names
}
