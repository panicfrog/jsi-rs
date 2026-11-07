use cxx::UniquePtr;

use jsi_tests::ffi::bridge::*;

pub fn create_raw_runtime() -> UniquePtr<jsi_sys::Runtime> {
    let config = create_runtime_config();
    let rt = create_hermes_runtime(&*config);
    let mut rt = cast_hermes_runtime(rt);
    // Newer Hermes built-ins (the InternalBytecode Promise machinery)
    // defer resolution through `setImmediate`, which a bare runtime
    // lacks — RN/metro hosts provide it. Run it inline: the promise
    // reactions it triggers still land on the microtask queue, which is
    // what the tests drain. Embedders wiring up their own runtime have
    // to provide this themselves (see README).
    drop(eval_js(
        rt.pin_mut(),
        "globalThis.setImmediate = fn => { fn(); };",
    ));
    rt
}
