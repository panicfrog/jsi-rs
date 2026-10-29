#[cxx::bridge]
pub mod ffi {
    #[namespace = "facebook::jsi"]
    unsafe extern "C++" {
        include!("jsi/jsi.h");
        include!("wrapper.h");

        pub type Buffer;

        fn size(self: &Buffer) -> usize;
        unsafe fn data(self: &Buffer) -> *const u8;

        pub type StringBuffer;

        pub type PreparedJavaScript;

        #[namespace = "jsi_rs::ffi"]
        pub type ConstPreparedJavaScript;

        #[namespace = "jsi_rs::ffi"]
        fn PreparedJavaScript_asConst(
            js: &SharedPtr<PreparedJavaScript>,
        ) -> SharedPtr<ConstPreparedJavaScript>;

        pub type Instrumentation;
        pub type Scope;
        pub type JSIException;
        pub type JSError;

        pub type Runtime;

        #[namespace = "jsi_rs::ffi"]
        pub fn Runtime_evaluateJavaScript(
            _self: Pin<&mut Runtime>,
            buffer: &SharedPtr<Buffer>,
            source_url: &str,
        ) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Runtime_prepareJavaScript(
            _self: Pin<&mut Runtime>,
            buffer: &SharedPtr<Buffer>,
            source_url: &str,
        ) -> SharedPtr<ConstPreparedJavaScript>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Runtime_evaluatePreparedJavaScript(
            _self: Pin<&mut Runtime>,
            js: &SharedPtr<ConstPreparedJavaScript>,
        ) -> UniquePtr<JsiValue>;
        #[cxx_name = "drainMicrotasks"]
        pub fn drain_microtasks(self: Pin<&mut Runtime>, max_microtasks_hint: i32) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn Runtime_global(_self: Pin<&mut Runtime>) -> UniquePtr<JsiObject>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Runtime_description(_self: Pin<&mut Runtime>) -> UniquePtr<CxxString>;
        #[cxx_name = "isInspectable"]
        pub fn is_inspectable(self: Pin<&mut Runtime>) -> bool;
        pub fn instrumentation(self: Pin<&mut Runtime>) -> Pin<&mut Instrumentation>;

        pub type HostObject;

        pub type NativeState;

        #[namespace = "jsi_rs::ffi"]
        pub fn Runtime_hasNativeState(
            rt: Pin<&mut Runtime>,
            obj: &JsiObject,
        ) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn Runtime_getNativeState(
            rt: Pin<&mut Runtime>,
            obj: &JsiObject,
        ) -> SharedPtr<NativeState>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_setNativeState(
            obj: &JsiObject,
            rt: Pin<&mut Runtime>,
            state: SharedPtr<NativeState>,
        );
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_createWithPrototype(
            rt: Pin<&mut Runtime>,
            prototype: &JsiValue,
        ) -> UniquePtr<JsiObject>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_setExternalMemoryPressure(
            obj: &JsiObject,
            rt: Pin<&mut Runtime>,
            amt: usize,
        );

        #[namespace = "jsi_rs::ffi"]
        pub fn HostObject_get(
            _self: Pin<&mut HostObject>,
            rt: Pin<&mut Runtime>,
            name: &PropNameID,
        ) -> UniquePtr<JsiValue>;
        pub fn set(
            self: Pin<&mut HostObject>,
            rt: Pin<&mut Runtime>,
            name: &PropNameID,
            value: &JsiValue,
        );
        #[namespace = "jsi_rs::ffi"]
        pub fn HostObject_getPropertyNames(
            _self: Pin<&mut HostObject>,
            rt: Pin<&mut Runtime>,
        ) -> UniquePtr<CxxVector<PropNameID>>;

        pub type Pointer;

        pub type PropNameID;

        #[namespace = "jsi_rs::ffi"]
        pub fn PropNameID_forUtf8(rt: Pin<&mut Runtime>, str: &str) -> UniquePtr<PropNameID>;
        #[namespace = "jsi_rs::ffi"]
        pub fn PropNameID_forString(
            rt: Pin<&mut Runtime>,
            str: &JsiString,
        ) -> UniquePtr<PropNameID>;
        #[namespace = "jsi_rs::ffi"]
        pub fn PropNameID_toUtf8(_self: &PropNameID, rt: Pin<&mut Runtime>)
            -> UniquePtr<CxxString>;
        #[namespace = "jsi_rs::ffi"]
        pub fn PropNameID_compare(
            rt: Pin<&mut Runtime>,
            lhs: &PropNameID,
            rhs: &PropNameID,
        ) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn PropNameID_copy(_self: &PropNameID, rt: Pin<&mut Runtime>) -> UniquePtr<PropNameID>;

        #[cxx_name = "Symbol"]
        pub type JsiSymbol;
        #[namespace = "jsi_rs::ffi"]
        pub fn Symbol_compare(rt: Pin<&mut Runtime>, lhs: &JsiSymbol, rhs: &JsiSymbol) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn Symbol_toString(_self: &JsiSymbol, rt: Pin<&mut Runtime>) -> UniquePtr<CxxString>;

        #[cxx_name = "String"]
        pub type JsiString;
        #[namespace = "jsi_rs::ffi"]
        pub fn String_fromUtf8(rt: Pin<&mut Runtime>, str: &str) -> UniquePtr<JsiString>;
        #[namespace = "jsi_rs::ffi"]
        pub fn String_compare(rt: Pin<&mut Runtime>, lhs: &JsiString, rhs: &JsiString) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn String_toString(_self: &JsiString, rt: Pin<&mut Runtime>) -> UniquePtr<CxxString>;

        #[cxx_name = "Object"]
        pub type JsiObject;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_create(rt: Pin<&mut Runtime>) -> UniquePtr<JsiObject>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_createFromHostObjectShared(
            rt: Pin<&mut Runtime>,
            ho: SharedPtr<HostObject>,
        ) -> UniquePtr<JsiObject>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_createFromHostObjectUnique(
            rt: Pin<&mut Runtime>,
            ho: UniquePtr<HostObject>,
        ) -> UniquePtr<JsiObject>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_compare(rt: Pin<&mut Runtime>, lhs: &JsiObject, rhs: &JsiObject) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_instanceOf(
            _self: &JsiObject,
            rt: Pin<&mut Runtime>,
            ctor: &JsiFunction,
        ) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_getProperty(
            _self: &JsiObject,
            rt: Pin<&mut Runtime>,
            prop: &PropNameID,
        ) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_hasProperty(
            _self: &JsiObject,
            rt: Pin<&mut Runtime>,
            prop: &PropNameID,
        ) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_setProperty(
            _self: Pin<&mut JsiObject>,
            rt: Pin<&mut Runtime>,
            prop: &PropNameID,
            value: &JsiValue,
        );
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_isArray(_self: &JsiObject, rt: Pin<&mut Runtime>) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_isArrayBuffer(_self: &JsiObject, rt: Pin<&mut Runtime>) -> bool;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_isFunction(_self: &JsiObject, rt: Pin<&mut Runtime>) -> bool;
        // TODO: isHostObject after implementing Rust HostObject subclass
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_asArray(
            _self: &JsiObject,
            rt: Pin<&mut Runtime>,
        ) -> Result<UniquePtr<JsiArray>>;
        // NOTICE: this method will assert if the object is not an array buffer;
        // i'm not sure if that's the same as an exception or whether it will
        // lead to UB
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_asArrayBuffer(
            _self: &JsiObject,
            rt: Pin<&mut Runtime>,
        ) -> Result<UniquePtr<JsiArrayBuffer>>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_asFunction(
            _self: &JsiObject,
            rt: Pin<&mut Runtime>,
        ) -> Result<UniquePtr<JsiFunction>>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_asHostObject(
            _self: &JsiObject,
            rt: Pin<&mut Runtime>,
        ) -> Result<SharedPtr<HostObject>>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Object_getPropertyNames(
            _self: Pin<&mut JsiObject>,
            rt: Pin<&mut Runtime>,
        ) -> UniquePtr<JsiArray>;

        #[cxx_name = "WeakObject"]
        pub type JsiWeakObject;
        #[namespace = "jsi_rs::ffi"]
        pub fn WeakObject_fromObject(
            rt: Pin<&mut Runtime>,
            object: &JsiObject,
        ) -> UniquePtr<JsiWeakObject>;
        #[namespace = "jsi_rs::ffi"]
        pub fn WeakObject_lock(
            _self: Pin<&mut JsiWeakObject>,
            rt: Pin<&mut Runtime>,
        ) -> UniquePtr<JsiValue>;

        #[cxx_name = "Array"]
        pub type JsiArray;
        #[namespace = "jsi_rs::ffi"]
        pub fn Array_createWithLength(rt: Pin<&mut Runtime>, length: usize) -> UniquePtr<JsiArray>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Array_get(
            _self: &JsiArray,
            rt: Pin<&mut Runtime>,
            index: usize,
        ) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Array_set(
            _self: Pin<&mut JsiArray>,
            rt: Pin<&mut Runtime>,
            index: usize,
            value: &JsiValue,
        );
        #[namespace = "jsi_rs::ffi"]
        pub fn Array_length(_self: &JsiArray, rt: Pin<&mut Runtime>) -> usize;

        #[cxx_name = "ArrayBuffer"]
        pub type JsiArrayBuffer;
        #[namespace = "jsi_rs::ffi"]
        pub unsafe fn ArrayBuffer_data(
            _self: &JsiArrayBuffer,
            rt: Pin<&mut Runtime>,
        ) -> *mut u8;
        #[namespace = "jsi_rs::ffi"]
        pub fn ArrayBuffer_length(_self: &JsiArrayBuffer, rt: Pin<&mut Runtime>) -> usize;
        /// Creates an ArrayBuffer backed by Rust-owned memory (`ctx` is a
        /// boxed Rust allocation; `drop_fn` frees it when the GC releases
        /// the buffer). Zero-copy Rust -> JS transfer.
        #[namespace = "jsi_rs::ffi"]
        pub unsafe fn Runtime_createArrayBufferFromRust(
            rt: Pin<&mut Runtime>,
            data: *mut u8,
            size: usize,
            ctx: *mut c_void,
            drop_fn: unsafe extern "C" fn(ctx: *mut c_void),
        ) -> UniquePtr<JsiArrayBuffer>;
        #[namespace = "jsi_rs::ffi"]
        pub unsafe fn Value_fromArrayBuffer(
            rt: Pin<&mut Runtime>,
            b: UniquePtr<JsiArrayBuffer>,
        ) -> UniquePtr<JsiValue>;

        #[cxx_name = "Function"]
        pub type JsiFunction;
        #[namespace = "jsi_rs::ffi"]
        pub fn Function_call(
            _self: &JsiFunction,
            rt: Pin<&mut Runtime>,
            args: &CxxVector<JsiValue>,
        ) -> Result<UniquePtr<JsiValue>>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Function_callAsConstructor(
            _self: &JsiFunction,
            rt: Pin<&mut Runtime>,
            args: &CxxVector<JsiValue>,
        ) -> Result<UniquePtr<JsiValue>>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Function_callWithThis(
            _self: &JsiFunction,
            rt: Pin<&mut Runtime>,
            thisObj: &JsiObject,
            args: &CxxVector<JsiValue>,
        ) -> Result<UniquePtr<JsiValue>>;
        #[namespace = "jsi_rs::ffi"]
        pub unsafe fn Function_createFromHostFunction(
            rt: Pin<&mut Runtime>,
            name: &PropNameID,
            param_count: u32,
            closure: *mut c_void,
        ) -> UniquePtr<JsiFunction>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Function_isHostFunction(_self: &JsiFunction, rt: Pin<&mut Runtime>) -> bool;

        #[cxx_name = "Value"]
        pub type JsiValue;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromUndefined() -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromNull() -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromBool(b: bool) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromDouble(d: f64) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromInt(i: i32) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromString(
            rt: Pin<&mut Runtime>,
            s: UniquePtr<JsiString>,
        ) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromObject(
            rt: Pin<&mut Runtime>,
            o: UniquePtr<JsiObject>,
        ) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromSymbol(
            rt: Pin<&mut Runtime>,
            s: UniquePtr<JsiSymbol>,
        ) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_copyFromString(rt: Pin<&mut Runtime>, s: &JsiString) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_copyFromObject(rt: Pin<&mut Runtime>, o: &JsiObject) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_copyFromSymbol(rt: Pin<&mut Runtime>, s: &JsiSymbol) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_fromJson(rt: Pin<&mut Runtime>, s: &str) -> UniquePtr<JsiValue>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_compare(rt: Pin<&mut Runtime>, lhs: &JsiValue, rhs: &JsiValue) -> bool;
        #[cxx_name = "isUndefined"]
        pub fn is_undefined(self: &JsiValue) -> bool;
        #[cxx_name = "isNull"]
        pub fn is_null(self: &JsiValue) -> bool;
        #[cxx_name = "isBool"]
        pub fn is_bool(self: &JsiValue) -> bool;
        #[cxx_name = "isNumber"]
        pub fn is_number(self: &JsiValue) -> bool;
        #[cxx_name = "isString"]
        pub fn is_string(self: &JsiValue) -> bool;
        #[cxx_name = "isSymbol"]
        pub fn is_symbol(self: &JsiValue) -> bool;
        #[cxx_name = "isObject"]
        pub fn is_object(self: &JsiValue) -> bool;
        #[cxx_name = "getBool"]
        pub fn get_bool(self: &JsiValue) -> Result<bool>;
        #[cxx_name = "asNumber"]
        pub fn get_number(self: &JsiValue) -> Result<f64>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_asString(
            _self: &JsiValue,
            rt: Pin<&mut Runtime>,
        ) -> Result<UniquePtr<JsiString>>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_asObject(
            _self: &JsiValue,
            rt: Pin<&mut Runtime>,
        ) -> Result<UniquePtr<JsiObject>>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_asSymbol(
            _self: &JsiValue,
            rt: Pin<&mut Runtime>,
        ) -> Result<UniquePtr<JsiSymbol>>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_toString(_self: &JsiValue, rt: Pin<&mut Runtime>) -> UniquePtr<JsiString>;
        #[namespace = "jsi_rs::ffi"]
        pub fn Value_copy(_self: &JsiValue, rt: Pin<&mut Runtime>) -> UniquePtr<JsiValue>;
    }

    impl UniquePtr<Runtime> {}


    #[namespace = "jsi_rs::ffi"]
    unsafe extern "C++" {
        pub type c_void;

        pub fn create_value_vector() -> UniquePtr<CxxVector<JsiValue>>;
        pub fn push_value_vector(vec: Pin<&mut CxxVector<JsiValue>>, item: UniquePtr<JsiValue>);
        pub fn create_prop_name_vector() -> UniquePtr<CxxVector<PropNameID>>;
        pub fn push_prop_name_vector(
            vec: Pin<&mut CxxVector<PropNameID>>,
            item: UniquePtr<PropNameID>,
        );
        pub fn pop_prop_name_vector(vec: Pin<&mut CxxVector<PropNameID>>) -> UniquePtr<PropNameID>;
    }

    #[namespace = "jsi_rs::ffi"]
    extern "Rust" {
        unsafe fn host_fn_trampoline(
            rt: Pin<&mut Runtime>,
            thisVal: &JsiValue,
            args: *const JsiValue,
            count: u32,
            stride: usize,
            closure: *mut c_void,
        ) -> Result<UniquePtr<JsiValue>>;
    }
}

pub use ffi::*;

pub type HostFunctionCallback<'rt> = Box<
    dyn FnMut(
            std::pin::Pin<&mut Runtime>,
            &JsiValue,
            &[&JsiValue],
        ) -> Result<cxx::UniquePtr<JsiValue>, anyhow::Error>
        + 'rt,
>;

unsafe fn host_fn_trampoline(
    rt: std::pin::Pin<&mut Runtime>,
    this: &JsiValue,
    args: *const JsiValue,
    count: u32,
    stride: usize,
    closure: *mut c_void,
) -> anyhow::Result<cxx::UniquePtr<JsiValue>> {
    let closure = closure as *mut HostFunctionCallback;
    let mut closure = Box::from_raw(closure);

    // Rust JsiValue type is just a marker type; its size according to Rust is
    // zero so we cannot construct a slice of JsiValue; instead the size of each
    // value is passed in from C++ and we do the pointer math ourselves

    let mut args_refs = Vec::with_capacity(count as usize);

    for i in 0..count {
        let ptr = (args as usize + stride * i as usize) as *const JsiValue;
        args_refs.push(&*ptr);
    }

    let res = closure(rt, this, &args_refs[..]);
    Box::leak(closure);
    res
}
