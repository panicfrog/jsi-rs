mod base;
mod host;

pub use base::*;
// the host bridge re-declares the shared types from the base bridge (Runtime,
// JsiValue, ...) which would collide in a glob import, so re-export its unique
// symbols explicitly
pub use host::{
    CxxHostObject, CxxHostObject_create, CxxHostObject_fromHostObjectS, CxxHostObject_fromHostObjectU,
    CxxHostObject_getInner, CxxHostObject_getInnerMut, CxxHostObject_toHostObjectS,
    CxxHostObject_toHostObjectU,
};
