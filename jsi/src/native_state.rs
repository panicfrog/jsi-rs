//! # Native state
//!
//! jsi `NativeState` lets a plain JS object carry an arbitrary native object
//! whose lifetime is bound to the JS GC ("live until the value at this
//! property becomes unreachable", per jsi.h). This is the Nitro-style
//! alternative to `HostObject`s: methods are pre-installed on a prototype and
//! `this` resolves through the object's native state.
//!
//! Note that `setNativeState` throws a type error on host objects and proxies
//! — only plain objects (e.g. created with [`JsiObject::create_with_prototype`])
//! can carry native state.

use std::marker::PhantomData;

use crate::{sys, JsiObject, JsiValue, RuntimeHandle};

/// A `jsi::NativeState` backed by a Rust object. Dropping the JS object's
/// native state drops the Rust object.
pub struct RustNativeStateHandle(cxx::SharedPtr<sys::RustNativeState>);

impl RustNativeStateHandle {
    /// The Rust object carried by this native state.
    pub fn get<T: std::any::Any + Send + Sync>(&self) -> Option<&T> {
        self.0
            .as_ref()
            .and_then(|s| sys::RustNativeState_getInner(s).0.downcast_ref::<T>())
    }
}

/// Creates a native state carrying the given Rust object.
pub fn rust_native_state<T: std::any::Any + Send + Sync>(value: T) -> cxx::SharedPtr<sys::NativeState> {
    sys::RustNativeState_create(Box::new(sys::RustNativeCarrier(Box::new(value))))
}

impl<'rt> JsiObject<'rt> {
    /// Creates a plain JS object whose prototype is the given value
    /// (`Object.create(prototype)`).
    pub fn create_with_prototype(
        prototype: &JsiValue<'_>,
        rt: &mut RuntimeHandle<'rt>,
    ) -> JsiObject<'rt> {
        JsiObject(
            sys::Object_createWithPrototype(rt.get_inner_mut(), prototype.0.as_ref().unwrap()),
            PhantomData,
        )
    }

    /// Whether this object carries any native state.
    pub fn has_native_state(&self, rt: &mut RuntimeHandle<'rt>) -> bool {
        sys::Runtime_hasNativeState(rt.get_inner_mut(), self.0.as_ref().unwrap())
    }

    /// Attaches a Rust object as this object's native state; the object's GC
    /// death will drop it. Throws if the object is a host object or proxy.
    pub fn set_rust_native_state<T: std::any::Any + Send + Sync>(
        &self,
        value: T,
        rt: &mut RuntimeHandle<'rt>,
    ) {
        sys::Object_setNativeState(
            self.0.as_ref().unwrap(),
            rt.get_inner_mut(),
            rust_native_state(value),
        )
    }

    /// The Rust object carried by this object's native state, if this object
    /// carries a Rust-backed one.
    pub fn get_rust_native_state(&self, rt: &mut RuntimeHandle<'rt>) -> Option<RustNativeStateHandle> {
        if !self.has_native_state(rt) {
            return None;
        }
        let ns = sys::Runtime_getNativeState(rt.get_inner_mut(), self.0.as_ref().unwrap());
        let rust = sys::NativeState_asRust(ns);
        if rust.is_null() {
            None
        } else {
            Some(RustNativeStateHandle(rust))
        }
    }

    /// Notifies the JS GC about native memory held by this object, so it is
    /// collected more eagerly.
    pub fn set_external_memory_pressure(&self, amt: usize, rt: &mut RuntimeHandle<'rt>) {
        sys::Object_setExternalMemoryPressure(self.0.as_ref().unwrap(), rt.get_inner_mut(), amt)
    }

    /// Detaches any Rust object from this object's native state, dropping it
    /// (the Nitro `dispose()` pattern).
    pub fn clear_native_state(&self, rt: &mut RuntimeHandle<'rt>) {
        sys::Object_setNativeState(
            self.0.as_ref().unwrap(),
            rt.get_inner_mut(),
            cxx::SharedPtr::null(),
        )
    }
}
