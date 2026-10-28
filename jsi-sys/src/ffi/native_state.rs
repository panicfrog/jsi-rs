use crate::shim::RustNativeCarrier;

#[cxx::bridge]
pub(crate) mod ffi {
    #[namespace = "jsi_rs::ffi"]
    unsafe extern "C++" {
        include!("native_state.h");

        // private alias so this bridge doesn't collide with the same type
        // exported from the base bridge when both modules are glob-imported
        #[namespace = "facebook::jsi"]
        type NativeState = crate::ffi::base::NativeState;

        pub type RustNativeState;

        pub fn RustNativeState_create(state: Box<RustNativeCarrier>) -> SharedPtr<NativeState>;
        pub fn NativeState_asRust(state: SharedPtr<NativeState>) -> SharedPtr<RustNativeState>;
        pub fn RustNativeState_getInner(ptr: &RustNativeState) -> &RustNativeCarrier;
    }

    #[namespace = "jsi_rs::ffi"]
    extern "Rust" {
        type RustNativeCarrier;
    }
}

pub use ffi::*;
