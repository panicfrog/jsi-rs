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
}
