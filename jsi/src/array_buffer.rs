use std::marker::PhantomData;

use crate::{sys, RuntimeHandle};

/// A JavaScript
/// [`ArrayBuffer`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/ArrayBuffer).
/// Can be used to share large buffers of data with a React Native application.
pub struct JsiArrayBuffer<'rt>(
    pub(crate) cxx::UniquePtr<sys::JsiArrayBuffer>,
    pub(crate) PhantomData<&'rt mut ()>,
);

impl<'rt> JsiArrayBuffer<'rt> {
    pub fn data(&self, rt: &mut RuntimeHandle<'rt>) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                sys::ArrayBuffer_data(&*self.0, rt.get_inner_mut()),
                sys::ArrayBuffer_length(&*self.0, rt.get_inner_mut()),
            )
        }
    }

    /// The buffer's size in bytes (not affected by overriding `byteLength`).
    pub fn len(&self, rt: &mut RuntimeHandle<'rt>) -> usize {
        sys::ArrayBuffer_length(&*self.0, rt.get_inner_mut())
    }

    /// Moves a Rust allocation into the runtime as an ArrayBuffer backed by
    /// that memory — a zero-copy handoff. The bytes are shared: JS writes
    /// are visible to Rust (and vice versa) until the GC releases the
    /// buffer, at which point the Rust allocation is dropped.
    pub fn from_vec(vec: Vec<u8>, rt: &mut RuntimeHandle<'rt>) -> Self {
        let mut vec = vec;
        let (ptr, len) = (vec.as_mut_ptr(), vec.len());
        // keep the allocation alive inside a box the C++ deleter will free
        let ctx = Box::into_raw(Box::new(vec)) as *mut sys::c_void;
        let raw = unsafe {
            sys::Runtime_createArrayBufferFromRust(
                rt.get_inner_mut(),
                ptr,
                len,
                ctx,
                drop_vec,
            )
        };
        JsiArrayBuffer(raw, PhantomData)
    }
}

fn drop_vec(ctx: *mut sys::c_void) {
    drop(unsafe { Box::from_raw(ctx as *mut Vec<u8>) });
}
