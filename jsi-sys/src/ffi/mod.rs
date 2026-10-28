#[cfg(target_os = "android")]
mod android;
mod base;
mod host;
mod native_state;

#[cfg(target_os = "android")]
pub use android::*;
pub use base::*;
// the host and native_state bridges re-declare shared types from the base
// bridge (Runtime, JsiValue, ...) which would collide in a glob import, so
// re-export their unique symbols explicitly
pub use host::{
    CxxHostObject, CxxHostObject_create, CxxHostObject_fromHostObjectS, CxxHostObject_fromHostObjectU,
    CxxHostObject_getInner, CxxHostObject_getInnerMut, CxxHostObject_toHostObjectS,
    CxxHostObject_toHostObjectU,
};
pub use native_state::{
    NativeState_asRust, RustNativeState, RustNativeState_create, RustNativeState_getInner,
};
