#[cxx::bridge]
pub mod ffi {
    #[namespace = "facebook::react"]
    unsafe extern "C++" {
        include!("rn_wrapper.h");

        pub type CallInvoker;
    }

    #[namespace = "jsi_rs::rn"]
    unsafe extern "C++" {
        pub type c_void;

        pub unsafe fn CallInvoker_invokeSync(_self: SharedPtr<CallInvoker>, closure: *mut c_void);
        pub unsafe fn CallInvoker_invokeAsync(_self: SharedPtr<CallInvoker>, closure: *mut c_void);
    }

    #[namespace = "jsi_rs::rn"]
    extern "Rust" {
        unsafe fn call_invoker_trampoline(closure: *mut c_void) -> Result<()>;
    }
}

pub use ffi::*;

unsafe impl Sync for CallInvoker {}
unsafe impl Send for CallInvoker {}

unsafe fn call_invoker_trampoline(closure: *mut c_void) -> anyhow::Result<()> {
    let closure = Box::from_raw(closure as *mut jsi::JsCallback);
    closure()
}
