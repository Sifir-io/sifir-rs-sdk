#[allow(unused_macros)]
macro_rules! swig_c_str {
    ($ lit : expr) => {
        concat!($lit, "\0").as_ptr() as *const ::std::os::raw::c_char
    };
}
#[allow(dead_code)]
pub trait SwigForeignClass {
    fn c_class_name() -> *const ::std::os::raw::c_char;
    fn box_object(x: Self) -> *mut ::std::os::raw::c_void;
    fn unbox_object(p: *mut ::std::os::raw::c_void) -> Self;
}
#[allow(dead_code)]
pub trait SwigForeignEnum {
    fn as_u32(&self) -> u32;
    fn from_u32(_: u32) -> Self;
}
#[allow(dead_code)]
#[doc = ""]
trait SwigInto<T> {
    fn swig_into(self) -> T;
}
#[allow(dead_code)]
#[doc = ""]
trait SwigFrom<T> {
    fn swig_from(_: T) -> Self;
}
#[allow(dead_code)]
#[doc = ""]
trait SwigDeref {
    type Target: ?Sized;
    fn swig_deref(&self) -> &Self::Target;
}
#[allow(dead_code)]
#[doc = ""]
trait SwigDerefMut {
    type Target: ?Sized;
    fn swig_deref_mut(&mut self) -> &mut Self::Target;
}
#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CRustStrView {
    data: *const ::std::os::raw::c_char,
    len: usize,
}
#[allow(dead_code)]
impl CRustStrView {
    fn from_str(s: &str) -> CRustStrView {
        CRustStrView {
            data: s.as_ptr() as *const ::std::os::raw::c_char,
            len: s.len(),
        }
    }
}
#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CRustString {
    data: *const ::std::os::raw::c_char,
    len: usize,
    capacity: usize,
}
#[allow(dead_code)]
impl CRustString {
    pub fn from_string(s: String) -> CRustString {
        let data = s.as_ptr() as *const ::std::os::raw::c_char;
        let len = s.len();
        let capacity = s.capacity();
        ::std::mem::forget(s);
        CRustString {
            data,
            len,
            capacity,
        }
    }
}
#[allow(dead_code)]
#[repr(C)]
pub struct CRustObjectSlice {
    data: *const ::std::os::raw::c_void,
    len: usize,
    step: usize,
}
#[allow(dead_code)]
#[repr(C)]
pub struct CRustObjectMutSlice {
    data: *mut ::std::os::raw::c_void,
    len: usize,
    step: usize,
}
#[allow(dead_code)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CRustForeignVec {
    data: *const ::std::os::raw::c_void,
    len: usize,
    capacity: usize,
    step: usize,
}
#[allow(dead_code)]
impl CRustForeignVec {
    pub fn from_vec<T: SwigForeignClass>(mut v: Vec<T>) -> CRustForeignVec {
        let data = v.as_mut_ptr() as *const ::std::os::raw::c_void;
        let len = v.len();
        let capacity = v.capacity();
        ::std::mem::forget(v);
        CRustForeignVec {
            data,
            len,
            capacity,
            step: ::std::mem::size_of::<T>(),
        }
    }
}
#[allow(dead_code)]
#[inline]
fn push_foreign_class_to_vec<T: SwigForeignClass>(
    vec: *mut CRustForeignVec,
    elem: *mut ::std::os::raw::c_void,
) {
    assert!(!vec.is_null());
    let vec: &mut CRustForeignVec = unsafe { &mut *vec };
    assert!(vec.len == 0 || ::std::mem::size_of::<T>() == vec.step);
    vec.step = ::std::mem::size_of::<T>();
    let mut v = unsafe { Vec::from_raw_parts(vec.data as *mut T, vec.len, vec.capacity) };
    v.push(T::unbox_object(elem));
    vec.data = v.as_mut_ptr() as *const ::std::os::raw::c_void;
    vec.len = v.len();
    vec.capacity = v.capacity();
    ::std::mem::forget(v);
}
#[allow(dead_code)]
#[inline]
fn remove_foreign_class_from_vec<T: SwigForeignClass>(
    vec: *mut CRustForeignVec,
    index: usize,
) -> *mut ::std::os::raw::c_void {
    assert!(!vec.is_null());
    let vec: &mut CRustForeignVec = unsafe { &mut *vec };
    assert_eq!(::std::mem::size_of::<T>(), vec.step);
    let mut v = unsafe { Vec::from_raw_parts(vec.data as *mut T, vec.len, vec.capacity) };
    let elem: T = v.remove(index);
    vec.data = v.as_mut_ptr() as *const ::std::os::raw::c_void;
    vec.len = v.len();
    vec.capacity = v.capacity();
    ::std::mem::forget(v);
    T::box_object(elem)
}
#[allow(dead_code)]
#[inline]
fn drop_foreign_class_vec<T: SwigForeignClass>(v: CRustForeignVec) {
    assert_eq!(::std::mem::size_of::<T>(), v.step);
    let v = unsafe { Vec::from_raw_parts(v.data as *mut T, v.len, v.capacity) };
    drop(v);
}
use jni_sys::*;
use log::*;
use logger::{log, Logger};
use serde_json::json;
use std::time::Duration;
#[cfg(feature = "tor_daemon")]
use tor::{
    hidden_service::{HiddenServiceDataHandler, HiddenServiceHandler},
    tcp_stream::{DataObserver, TcpSocksStream},
    BootstrapPhase, OwnedTorService, OwnedTorServiceBootstrapPhase, TorHiddenService,
    TorHiddenServiceParam, TorServiceParam,
};
unsafe impl Send for Observer {}
unsafe impl Sync for Observer {}
struct Observer {
    cb: Box<dyn DataObserver>,
}
impl DataObserver for Observer {
    fn on_data(&self, data: String) {
        self.cb.on_data(data);
    }
    fn on_error(&self, data: String) {
        self.cb.on_error(data);
    }
}
#[allow(non_snake_case)]
#[test]
fn test_CRustStrView_layout() {
    #[repr(C)]
    struct MyCRustStrView {
        data: *const ::std::os::raw::c_char,
        len: usize,
    }
    assert_eq!(
        ::std::mem::size_of::<MyCRustStrView>(),
        ::std::mem::size_of::<CRustStrView>()
    );
    assert_eq!(
        ::std::mem::align_of::<MyCRustStrView>(),
        ::std::mem::align_of::<CRustStrView>()
    );
    let our_s: MyCRustStrView = unsafe { ::std::mem::zeroed() };
    let user_s: CRustStrView = unsafe { ::std::mem::zeroed() };
    #[allow(dead_code)]
    fn check_CRustStrView_data_type_fn(s: &CRustStrView) -> &*const ::std::os::raw::c_char {
        &s.data
    }
    let offset_our = ((&our_s.data as *const *const ::std::os::raw::c_char) as usize)
        - ((&our_s as *const MyCRustStrView) as usize);
    let offset_user = ((&user_s.data as *const *const ::std::os::raw::c_char) as usize)
        - ((&user_s as *const CRustStrView) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustStrView_len_type_fn(s: &CRustStrView) -> &usize {
        &s.len
    }
    let offset_our =
        ((&our_s.len as *const usize) as usize) - ((&our_s as *const MyCRustStrView) as usize);
    let offset_user =
        ((&user_s.len as *const usize) as usize) - ((&user_s as *const CRustStrView) as usize);
    assert_eq!(offset_our, offset_user);
}
#[allow(non_snake_case)]
#[test]
fn test_CRustString_layout() {
    #[repr(C)]
    struct MyCRustString {
        data: *const ::std::os::raw::c_char,
        len: usize,
        capacity: usize,
    }
    assert_eq!(
        ::std::mem::size_of::<MyCRustString>(),
        ::std::mem::size_of::<CRustString>()
    );
    assert_eq!(
        ::std::mem::align_of::<MyCRustString>(),
        ::std::mem::align_of::<CRustString>()
    );
    let our_s: MyCRustString = unsafe { ::std::mem::zeroed() };
    let user_s: CRustString = unsafe { ::std::mem::zeroed() };
    #[allow(dead_code)]
    fn check_CRustString_data_type_fn(s: &CRustString) -> &*const ::std::os::raw::c_char {
        &s.data
    }
    let offset_our = ((&our_s.data as *const *const ::std::os::raw::c_char) as usize)
        - ((&our_s as *const MyCRustString) as usize);
    let offset_user = ((&user_s.data as *const *const ::std::os::raw::c_char) as usize)
        - ((&user_s as *const CRustString) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustString_len_type_fn(s: &CRustString) -> &usize {
        &s.len
    }
    let offset_our =
        ((&our_s.len as *const usize) as usize) - ((&our_s as *const MyCRustString) as usize);
    let offset_user =
        ((&user_s.len as *const usize) as usize) - ((&user_s as *const CRustString) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustString_capacity_type_fn(s: &CRustString) -> &usize {
        &s.capacity
    }
    let offset_our =
        ((&our_s.capacity as *const usize) as usize) - ((&our_s as *const MyCRustString) as usize);
    let offset_user =
        ((&user_s.capacity as *const usize) as usize) - ((&user_s as *const CRustString) as usize);
    assert_eq!(offset_our, offset_user);
}
#[no_mangle]
pub extern "C" fn crust_string_free(x: CRustString) {
    let s = unsafe { String::from_raw_parts(x.data as *mut u8, x.len, x.capacity) };
    drop(s);
}
#[no_mangle]
pub extern "C" fn crust_string_clone(x: CRustString) -> CRustString {
    let s = unsafe { String::from_raw_parts(x.data as *mut u8, x.len, x.capacity) };
    let ret = CRustString::from_string(s.clone());
    ::std::mem::forget(s);
    ret
}
#[allow(non_snake_case)]
#[test]
fn test_CRustObjectSlice_layout() {
    #[repr(C)]
    struct MyCRustObjectSlice {
        data: *const ::std::os::raw::c_void,
        len: usize,
        step: usize,
    }
    assert_eq!(
        ::std::mem::size_of::<MyCRustObjectSlice>(),
        ::std::mem::size_of::<CRustObjectSlice>()
    );
    assert_eq!(
        ::std::mem::align_of::<MyCRustObjectSlice>(),
        ::std::mem::align_of::<CRustObjectSlice>()
    );
    let our_s: MyCRustObjectSlice = unsafe { ::std::mem::zeroed() };
    let user_s: CRustObjectSlice = unsafe { ::std::mem::zeroed() };
    #[allow(dead_code)]
    fn check_CRustObjectSlice_data_type_fn(s: &CRustObjectSlice) -> &*const ::std::os::raw::c_void {
        &s.data
    }
    let offset_our = ((&our_s.data as *const *const ::std::os::raw::c_void) as usize)
        - ((&our_s as *const MyCRustObjectSlice) as usize);
    let offset_user = ((&user_s.data as *const *const ::std::os::raw::c_void) as usize)
        - ((&user_s as *const CRustObjectSlice) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustObjectSlice_len_type_fn(s: &CRustObjectSlice) -> &usize {
        &s.len
    }
    let offset_our =
        ((&our_s.len as *const usize) as usize) - ((&our_s as *const MyCRustObjectSlice) as usize);
    let offset_user =
        ((&user_s.len as *const usize) as usize) - ((&user_s as *const CRustObjectSlice) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustObjectSlice_step_type_fn(s: &CRustObjectSlice) -> &usize {
        &s.step
    }
    let offset_our =
        ((&our_s.step as *const usize) as usize) - ((&our_s as *const MyCRustObjectSlice) as usize);
    let offset_user =
        ((&user_s.step as *const usize) as usize) - ((&user_s as *const CRustObjectSlice) as usize);
    assert_eq!(offset_our, offset_user);
}
#[allow(non_snake_case)]
#[test]
fn test_CRustObjectMutSlice_layout() {
    #[repr(C)]
    struct MyCRustObjectMutSlice {
        data: *mut ::std::os::raw::c_void,
        len: usize,
        step: usize,
    }
    assert_eq!(
        ::std::mem::size_of::<MyCRustObjectMutSlice>(),
        ::std::mem::size_of::<CRustObjectMutSlice>()
    );
    assert_eq!(
        ::std::mem::align_of::<MyCRustObjectMutSlice>(),
        ::std::mem::align_of::<CRustObjectMutSlice>()
    );
    let our_s: MyCRustObjectMutSlice = unsafe { ::std::mem::zeroed() };
    let user_s: CRustObjectMutSlice = unsafe { ::std::mem::zeroed() };
    #[allow(dead_code)]
    fn check_CRustObjectMutSlice_data_type_fn(
        s: &CRustObjectMutSlice,
    ) -> &*mut ::std::os::raw::c_void {
        &s.data
    }
    let offset_our = ((&our_s.data as *const *mut ::std::os::raw::c_void) as usize)
        - ((&our_s as *const MyCRustObjectMutSlice) as usize);
    let offset_user = ((&user_s.data as *const *mut ::std::os::raw::c_void) as usize)
        - ((&user_s as *const CRustObjectMutSlice) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustObjectMutSlice_len_type_fn(s: &CRustObjectMutSlice) -> &usize {
        &s.len
    }
    let offset_our = ((&our_s.len as *const usize) as usize)
        - ((&our_s as *const MyCRustObjectMutSlice) as usize);
    let offset_user = ((&user_s.len as *const usize) as usize)
        - ((&user_s as *const CRustObjectMutSlice) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustObjectMutSlice_step_type_fn(s: &CRustObjectMutSlice) -> &usize {
        &s.step
    }
    let offset_our = ((&our_s.step as *const usize) as usize)
        - ((&our_s as *const MyCRustObjectMutSlice) as usize);
    let offset_user = ((&user_s.step as *const usize) as usize)
        - ((&user_s as *const CRustObjectMutSlice) as usize);
    assert_eq!(offset_our, offset_user);
}
#[allow(non_snake_case)]
#[test]
fn test_CRustForeignVec_layout() {
    #[repr(C)]
    struct MyCRustForeignVec {
        data: *const ::std::os::raw::c_void,
        len: usize,
        capacity: usize,
        step: usize,
    }
    assert_eq!(
        ::std::mem::size_of::<MyCRustForeignVec>(),
        ::std::mem::size_of::<CRustForeignVec>()
    );
    assert_eq!(
        ::std::mem::align_of::<MyCRustForeignVec>(),
        ::std::mem::align_of::<CRustForeignVec>()
    );
    let our_s: MyCRustForeignVec = unsafe { ::std::mem::zeroed() };
    let user_s: CRustForeignVec = unsafe { ::std::mem::zeroed() };
    #[allow(dead_code)]
    fn check_CRustForeignVec_data_type_fn(s: &CRustForeignVec) -> &*const ::std::os::raw::c_void {
        &s.data
    }
    let offset_our = ((&our_s.data as *const *const ::std::os::raw::c_void) as usize)
        - ((&our_s as *const MyCRustForeignVec) as usize);
    let offset_user = ((&user_s.data as *const *const ::std::os::raw::c_void) as usize)
        - ((&user_s as *const CRustForeignVec) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustForeignVec_len_type_fn(s: &CRustForeignVec) -> &usize {
        &s.len
    }
    let offset_our =
        ((&our_s.len as *const usize) as usize) - ((&our_s as *const MyCRustForeignVec) as usize);
    let offset_user =
        ((&user_s.len as *const usize) as usize) - ((&user_s as *const CRustForeignVec) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustForeignVec_capacity_type_fn(s: &CRustForeignVec) -> &usize {
        &s.capacity
    }
    let offset_our = ((&our_s.capacity as *const usize) as usize)
        - ((&our_s as *const MyCRustForeignVec) as usize);
    let offset_user = ((&user_s.capacity as *const usize) as usize)
        - ((&user_s as *const CRustForeignVec) as usize);
    assert_eq!(offset_our, offset_user);
    #[allow(dead_code)]
    fn check_CRustForeignVec_step_type_fn(s: &CRustForeignVec) -> &usize {
        &s.step
    }
    let offset_our =
        ((&our_s.step as *const usize) as usize) - ((&our_s as *const MyCRustForeignVec) as usize);
    let offset_user =
        ((&user_s.step as *const usize) as usize) - ((&user_s as *const CRustForeignVec) as usize);
    assert_eq!(offset_our, offset_user);
}
impl SwigForeignClass for Logger {
    fn c_class_name() -> *const ::std::os::raw::c_char {
        swig_c_str!(stringify!(Logger))
    }
    fn box_object(this: Self) -> *mut ::std::os::raw::c_void {
        let this: Box<Logger> = Box::new(this);
        let this: *mut Logger = Box::into_raw(this);
        this as *mut ::std::os::raw::c_void
    }
    fn unbox_object(p: *mut ::std::os::raw::c_void) -> Self {
        let p = p as *mut Logger;
        let p: Box<Logger> = unsafe { Box::from_raw(p) };
        let p: Logger = *p;
        p
    }
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn Logger_new() -> *mut ::std::os::raw::c_void {
    let mut ret: Logger = Logger::new();
    let ret: *mut ::std::os::raw::c_void = <Logger>::box_object(ret);
    ret
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn Logger_delete(this: *mut Logger) {
    let this: Box<Logger> = unsafe { Box::from_raw(this) };
    drop(this);
}
#[repr(C)]
#[derive(Clone)]
#[allow(non_snake_case)]
pub struct C_DataObserver {
    opaque: *const ::std::os::raw::c_void,
    C_DataObserver_deref: extern "C" fn(_: *const ::std::os::raw::c_void),
    onData: extern "C" fn(a0: CRustString, _: *const ::std::os::raw::c_void) -> (),
    onError: extern "C" fn(a0: CRustString, _: *const ::std::os::raw::c_void) -> (),
}
#[doc = " It totally depends on С++ implementation"]
#[doc = " let's assume it safe"]
unsafe impl Send for C_DataObserver {}
impl DataObserver for C_DataObserver {
    #[allow(unused_mut)]
    fn on_data(&self, a0: String) -> () {
        let mut a0: CRustString = CRustString::from_string(a0);
        let ret: () = (self.onData)(a0, self.opaque);
        ret
    }
    #[allow(unused_mut)]
    fn on_error(&self, a0: String) -> () {
        let mut a0: CRustString = CRustString::from_string(a0);
        let ret: () = (self.onError)(a0, self.opaque);
        ret
    }
}
impl Drop for C_DataObserver {
    fn drop(&mut self) {
        (self.C_DataObserver_deref)(self.opaque);
    }
}
impl SwigForeignClass for Result<HiddenServiceHandler, String> {
    fn c_class_name() -> *const ::std::os::raw::c_char {
        swig_c_str!(stringify ! (Result < HiddenServiceHandler , String >))
    }
    fn box_object(this: Self) -> *mut ::std::os::raw::c_void {
        let this: Box<Result<HiddenServiceHandler, String>> = Box::new(this);
        let this: *mut Result<HiddenServiceHandler, String> = Box::into_raw(this);
        this as *mut ::std::os::raw::c_void
    }
    fn unbox_object(p: *mut ::std::os::raw::c_void) -> Self {
        let p = p as *mut Result<HiddenServiceHandler, String>;
        let p: Box<Result<HiddenServiceHandler, String>> = unsafe { Box::from_raw(p) };
        let p: Result<HiddenServiceHandler, String> = *p;
        p
    }
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn HiddenServiceHandler_new(
    dst_port: u16,
    cb: *const C_DataObserver,
) -> *const ::std::os::raw::c_void {
    assert!(!cb.is_null());
    let cb: &C_DataObserver = unsafe { cb.as_ref().unwrap() };
    let cb: Box<dyn DataObserver> = Box::new(cb.clone());
    let this: Result<HiddenServiceHandler, String> = {
        let mut lsnr = HiddenServiceHandler::new(dst_port)
            .map_err(|e| format!("{:#?}", e))
            .unwrap();
        lsnr.set_data_handler(Observer { cb })
            .map_err(|e| format!("{:#?}", e))
            .unwrap();
        let _ = lsnr.start_http_listener();
        Ok(lsnr)
    };
    let this: Box<Result<HiddenServiceHandler, String>> = Box::new(this);
    let this: *mut Result<HiddenServiceHandler, String> = Box::into_raw(this);
    this as *const ::std::os::raw::c_void
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn HiddenServiceHandler_delete(this: *mut Result<HiddenServiceHandler, String>) {
    let this: Box<Result<HiddenServiceHandler, String>> = unsafe { Box::from_raw(this) };
    drop(this);
}
impl SwigForeignClass for TorHiddenService {
    fn c_class_name() -> *const ::std::os::raw::c_char {
        swig_c_str!(stringify!(TorHiddenService))
    }
    fn box_object(this: Self) -> *mut ::std::os::raw::c_void {
        let this: Box<TorHiddenService> = Box::new(this);
        let this: *mut TorHiddenService = Box::into_raw(this);
        this as *mut ::std::os::raw::c_void
    }
    fn unbox_object(p: *mut ::std::os::raw::c_void) -> Self {
        let p = p as *mut TorHiddenService;
        let p: Box<TorHiddenService> = unsafe { Box::from_raw(p) };
        let p: TorHiddenService = *p;
        p
    }
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TorHiddenService_get_onion_url(this: *mut TorHiddenService) -> CRustString {
    let this: &TorHiddenService = unsafe { this.as_mut().unwrap() };
    let mut ret: String = { this.onion_url.to_string() };
    let mut ret: CRustString = CRustString::from_string(ret);
    ret
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TorHiddenService_get_secret_b64(this: *mut TorHiddenService) -> CRustString {
    let this: &TorHiddenService = unsafe { this.as_mut().unwrap() };
    let mut ret: String = { base64::encode(this.secret_key).into() };
    let mut ret: CRustString = CRustString::from_string(ret);
    ret
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TorHiddenService_delete(this: *mut TorHiddenService) {
    let this: Box<TorHiddenService> = unsafe { Box::from_raw(this) };
    drop(this);
}
impl SwigForeignClass for TorServiceParam {
    fn c_class_name() -> *const ::std::os::raw::c_char {
        swig_c_str!(stringify!(TorServiceParam))
    }
    fn box_object(this: Self) -> *mut ::std::os::raw::c_void {
        let this: Box<TorServiceParam> = Box::new(this);
        let this: *mut TorServiceParam = Box::into_raw(this);
        this as *mut ::std::os::raw::c_void
    }
    fn unbox_object(p: *mut ::std::os::raw::c_void) -> Self {
        let p = p as *mut TorServiceParam;
        let p: Box<TorServiceParam> = unsafe { Box::from_raw(p) };
        let p: TorServiceParam = *p;
        p
    }
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TorServiceParam_new(
    data_dir: CRustStrView,
    socks_port: u16,
    bootstap_timeout_ms: u64,
) -> *const ::std::os::raw::c_void {
    let mut data_dir: &str = unsafe {
        let slice: &[u8] = ::std::slice::from_raw_parts(data_dir.data as *const u8, data_dir.len);
        ::std::str::from_utf8_unchecked(slice)
    };
    let this: TorServiceParam = TorServiceParam::new(data_dir, socks_port, bootstap_timeout_ms);
    let this: Box<TorServiceParam> = Box::new(this);
    let this: *mut TorServiceParam = Box::into_raw(this);
    this as *const ::std::os::raw::c_void
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TorServiceParam_delete(this: *mut TorServiceParam) {
    let this: Box<TorServiceParam> = unsafe { Box::from_raw(this) };
    drop(this);
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union CRustVoidOkResultUnionCRustString {
    ok: u8,
    err: CRustString,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CRustVoidOkResultCRustString {
    data: CRustVoidOkResultUnionCRustString,
    is_ok: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union CRustResultUnion4232mut3232c_voidCRustString {
    ok: *mut ::std::os::raw::c_void,
    err: CRustString,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CRustResult4232mut3232c_voidCRustString {
    data: CRustResultUnion4232mut3232c_voidCRustString,
    is_ok: u8,
}
impl SwigForeignClass for OwnedTorService {
    fn c_class_name() -> *const ::std::os::raw::c_char {
        swig_c_str!(stringify!(OwnedTorService))
    }
    fn box_object(this: Self) -> *mut ::std::os::raw::c_void {
        let this: Box<OwnedTorService> = Box::new(this);
        let this: *mut OwnedTorService = Box::into_raw(this);
        this as *mut ::std::os::raw::c_void
    }
    fn unbox_object(p: *mut ::std::os::raw::c_void) -> Self {
        let p = p as *mut OwnedTorService;
        let p: Box<OwnedTorService> = unsafe { Box::from_raw(p) };
        let p: OwnedTorService = *p;
        p
    }
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn OwnedTorService_new(
    param: *mut ::std::os::raw::c_void,
) -> *const ::std::os::raw::c_void {
    assert!(!param.is_null());
    let param: *mut TorServiceParam = param as *mut TorServiceParam;
    let param: Box<TorServiceParam> = unsafe { Box::from_raw(param) };
    let param: TorServiceParam = *param;
    let this: OwnedTorService = {
        Logger::new();
        OwnedTorService::new(param).unwrap()
    };
    let this: Box<OwnedTorService> = Box::new(this);
    let this: *mut OwnedTorService = Box::into_raw(this);
    this as *const ::std::os::raw::c_void
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn OwnedTorService_getSocksPort(this: *mut OwnedTorService) -> u16 {
    let this: &OwnedTorService = unsafe { this.as_mut().unwrap() };
    let mut ret: u16 = { this.socks_port };
    ret
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn OwnedTorService_shutdown(
    this: *mut OwnedTorService,
) -> CRustVoidOkResultCRustString {
    let this: &mut OwnedTorService = unsafe { this.as_mut().unwrap() };
    let mut ret: Result<(), String> = { this.shutdown().map_err(|e| format!("{:#?}", e)) };
    let mut ret: CRustVoidOkResultCRustString = match ret {
        Ok(()) => CRustVoidOkResultCRustString {
            data: CRustVoidOkResultUnionCRustString { ok: 0 },
            is_ok: 1,
        },
        Err(err) => {
            let mut err: CRustString = CRustString::from_string(err);
            CRustVoidOkResultCRustString {
                data: CRustVoidOkResultUnionCRustString { err },
                is_ok: 0,
            }
        }
    };
    ret
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn OwnedTorService_get_status(this: *mut OwnedTorService) -> CRustString {
    let this: &mut OwnedTorService = unsafe { this.as_mut().unwrap() };
    let mut ret: String = {
        let node_status = this.get_status();
        match node_status {
            Ok(status) => {
                let status_string = serde_json::to_string(&status).unwrap();
                status_string
            }
            Err(e) => e.to_string(),
        }
    };
    let mut ret: CRustString = CRustString::from_string(ret);
    ret
}
impl<'a> SwigInto<String> for &'a str {
    fn swig_into(self) -> String {
        self.into()
    }
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn OwnedTorService_create_hidden_service(
    this: *mut OwnedTorService,
    dst_port: u16,
    hs_port: u16,
    secret_key: CRustStrView,
) -> CRustResult4232mut3232c_voidCRustString {
    let mut secret_key: &str = unsafe {
        let slice: &[u8] =
            ::std::slice::from_raw_parts(secret_key.data as *const u8, secret_key.len);
        ::std::str::from_utf8_unchecked(slice)
    };
    let mut secret_key: String = secret_key.swig_into();
    let this: &mut OwnedTorService = unsafe { this.as_mut().unwrap() };
    let mut ret: Result<TorHiddenService, String> = {
        let hs_key = match secret_key.len() {
            0 => Ok(None),
            _ => {
                let mut decoded_buff: [u8; 64] = [0; 64];
                base64::decode_config_slice(secret_key, base64::STANDARD, &mut decoded_buff)
                    .map(|_| Some(decoded_buff))
            }
        };
        match hs_key {
            Ok(key) => this
                .create_hidden_service(TorHiddenServiceParam {
                    to_port: dst_port,
                    hs_port,
                    secret_key: key,
                })
                .map_err(|e| format!("{:#?}", e)),
            Err(e) => Err(format!("{:#?}", e)),
        }
    };
    let mut ret: CRustResult4232mut3232c_voidCRustString = match ret {
        Ok(mut x) => {
            let ok: *mut ::std::os::raw::c_void = <TorHiddenService>::box_object(x);
            CRustResult4232mut3232c_voidCRustString {
                data: CRustResultUnion4232mut3232c_voidCRustString { ok },
                is_ok: 1,
            }
        }
        Err(err) => {
            let mut err: CRustString = CRustString::from_string(err);
            CRustResult4232mut3232c_voidCRustString {
                data: CRustResultUnion4232mut3232c_voidCRustString { err },
                is_ok: 0,
            }
        }
    };
    ret
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn OwnedTorService_delete_hidden_service(
    this: *mut OwnedTorService,
    onion: CRustStrView,
) -> CRustVoidOkResultCRustString {
    let mut onion: &str = unsafe {
        let slice: &[u8] = ::std::slice::from_raw_parts(onion.data as *const u8, onion.len);
        ::std::str::from_utf8_unchecked(slice)
    };
    let mut onion: String = onion.swig_into();
    let this: &mut OwnedTorService = unsafe { this.as_mut().unwrap() };
    let mut ret: Result<(), String> = {
        this.delete_hidden_service(onion)
            .map_err(|e| format!("{:#?}", e))
    };
    let mut ret: CRustVoidOkResultCRustString = match ret {
        Ok(()) => CRustVoidOkResultCRustString {
            data: CRustVoidOkResultUnionCRustString { ok: 0 },
            is_ok: 1,
        },
        Err(err) => {
            let mut err: CRustString = CRustString::from_string(err);
            CRustVoidOkResultCRustString {
                data: CRustVoidOkResultUnionCRustString { err },
                is_ok: 0,
            }
        }
    };
    ret
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn OwnedTorService_delete(this: *mut OwnedTorService) {
    let this: Box<OwnedTorService> = unsafe { Box::from_raw(this) };
    drop(this);
}
impl SwigForeignClass for TcpSocksStream {
    fn c_class_name() -> *const ::std::os::raw::c_char {
        swig_c_str!(stringify!(TcpSocksStream))
    }
    fn box_object(this: Self) -> *mut ::std::os::raw::c_void {
        let this: Box<TcpSocksStream> = Box::new(this);
        let this: *mut TcpSocksStream = Box::into_raw(this);
        this as *mut ::std::os::raw::c_void
    }
    fn unbox_object(p: *mut ::std::os::raw::c_void) -> Self {
        let p = p as *mut TcpSocksStream;
        let p: Box<TcpSocksStream> = unsafe { Box::from_raw(p) };
        let p: TcpSocksStream = *p;
        p
    }
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TcpSocksStream_new(
    target: CRustStrView,
    socks_proxy: CRustStrView,
    timeout_ms: u64,
) -> *const ::std::os::raw::c_void {
    let mut target: &str = unsafe {
        let slice: &[u8] = ::std::slice::from_raw_parts(target.data as *const u8, target.len);
        ::std::str::from_utf8_unchecked(slice)
    };
    let mut target: String = target.swig_into();
    let mut socks_proxy: &str = unsafe {
        let slice: &[u8] =
            ::std::slice::from_raw_parts(socks_proxy.data as *const u8, socks_proxy.len);
        ::std::str::from_utf8_unchecked(slice)
    };
    let mut socks_proxy: String = socks_proxy.swig_into();
    let this: TcpSocksStream =
        { TcpSocksStream::new_timeout(target, socks_proxy, timeout_ms).unwrap() };
    let this: Box<TcpSocksStream> = Box::new(this);
    let this: *mut TcpSocksStream = Box::into_raw(this);
    this as *const ::std::os::raw::c_void
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TcpSocksStream_on_data(
    this: *mut TcpSocksStream,
    cb: *const C_DataObserver,
) -> CRustVoidOkResultCRustString {
    assert!(!cb.is_null());
    let cb: &C_DataObserver = unsafe { cb.as_ref().unwrap() };
    let cb: Box<dyn DataObserver> = Box::new(cb.clone());
    let this: &mut TcpSocksStream = unsafe { this.as_mut().unwrap() };
    let mut ret: Result<(), String> = {
        this.set_data_handler(Observer { cb }).unwrap();
        this.read_line_async().map_err(|e| format!("{:#?}", e))
    };
    let mut ret: CRustVoidOkResultCRustString = match ret {
        Ok(()) => CRustVoidOkResultCRustString {
            data: CRustVoidOkResultUnionCRustString { ok: 0 },
            is_ok: 1,
        },
        Err(err) => {
            let mut err: CRustString = CRustString::from_string(err);
            CRustVoidOkResultCRustString {
                data: CRustVoidOkResultUnionCRustString { err },
                is_ok: 0,
            }
        }
    };
    ret
}
#[allow(non_snake_case, unused_variables, unused_mut, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TcpSocksStream_send_data(
    this: *mut TcpSocksStream,
    msg: CRustStrView,
    timeout: u64,
) -> CRustVoidOkResultCRustString {
    let mut msg: &str = unsafe {
        let slice: &[u8] = ::std::slice::from_raw_parts(msg.data as *const u8, msg.len);
        ::std::str::from_utf8_unchecked(slice)
    };
    let mut msg: String = msg.swig_into();
    let this: &mut TcpSocksStream = unsafe { this.as_mut().unwrap() };
    let mut ret: Result<(), String> = {
        this.send_data(msg, Some(Duration::new(timeout, 0)))
            .map_err(|e| format!("{:#?}", e))
    };
    let mut ret: CRustVoidOkResultCRustString = match ret {
        Ok(()) => CRustVoidOkResultCRustString {
            data: CRustVoidOkResultUnionCRustString { ok: 0 },
            is_ok: 1,
        },
        Err(err) => {
            let mut err: CRustString = CRustString::from_string(err);
            CRustVoidOkResultCRustString {
                data: CRustVoidOkResultUnionCRustString { err },
                is_ok: 0,
            }
        }
    };
    ret
}
#[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
#[no_mangle]
pub extern "C" fn TcpSocksStream_delete(this: *mut TcpSocksStream) {
    let this: Box<TcpSocksStream> = unsafe { Box::from_raw(this) };
    drop(this);
}
