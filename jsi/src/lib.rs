use std::{future::Future, pin::Pin};

#[cfg(feature = "macros")]
pub use jsi_macros::{host_object, HostEvent};
pub use jsi_sys as sys;

// allows us to use the proc macros inside this crate
extern crate self as jsi;

mod array;
mod array_buffer;
#[cfg(feature = "serde")]
mod convert;
mod function;
mod host_event;
mod host_function;
mod host_object;
mod object;
mod prop_name;
mod runtime;
mod scheduler;
mod string;
mod symbol;
mod value;

pub use array::*;
pub use array_buffer::*;
#[cfg(feature = "serde")]
pub use convert::*;
pub use function::*;
pub use host_event::*;
pub use host_function::*;
pub use host_object::*;
pub use object::*;
pub use prop_name::*;
pub use runtime::*;
pub use scheduler::*;
pub use string::*;
pub use symbol::*;
pub use value::*;

/// Creates a JavaScript `Error` object with the given string as the message.
#[macro_export]
macro_rules! js_error {
    ($rt: expr, $err: tt) => {{
        let mut rt = $rt;
        let error_ctor = rt.global().get(::jsi::PropName::new("Error", rt), rt);
        let error_ctor: ::jsi::JsiFn = error_ctor.try_into_js(rt).unwrap();
        let error_str = format!($err);
        let error_str = ::jsi::JsiValue::new_string(error_str.as_str(), rt);
        let error = error_ctor
            .call_as_constructor(::std::iter::once(error_str), rt)
            .expect("Error constructor threw an exception");
        let error: ::jsi::JsiObject = error
            .try_into_js(rt)
            .expect("Error constructor returned a non-object");
        error
    }};
}

pub type JsTaskCallback = Box<
    dyn (for<'a> FnOnce(
            &'a mut RuntimeHandle<'a>,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>>)
        + Send,
>;

/// Wraps a raw `jsi::Runtime` pointer handed over by the host.
///
/// The caller must ensure that the runtime outlives every JSI object created
/// from the returned handle, and that the handle is only used on the runtime's
/// JS thread. React Native hosts should use `jsi_rn::init`, which also wraps
/// the CallInvoker.
pub fn init(rt: *mut sys::Runtime) -> RuntimeHandle<'static> {
    #[cfg(feature = "log")]
    log::debug!("got JSI runtime pointer: {:p}", rt);

    RuntimeHandle::new_unchecked(rt)
}
