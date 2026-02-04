#![allow(dead_code, unused_imports, unused_macros)]
pub mod client;
pub use client::{LspClient, JsonRpcMessage};
pub mod tiec;

use std::ffi::c_void;
use std::ffi::NulError;
use std::os::raw::c_char;

/// DLL 侧返回的句柄类型
type TcHandle = isize;
/// DLL 侧返回的错误码类型
type TcError = i32;

#[cfg(windows)]
unsafe extern "C" {
    fn tc_guarded_ide_service_compile_files(
        func_ptr: *const c_void,
        ide_service_handle: TcHandle,
        file_count: usize,
        files: *const *const c_char,
        out_code: *mut TcError,
    ) -> u32;

    fn tc_guarded_free_handle(
        func_ptr: *const c_void,
        handle: TcHandle,
        out_code: *mut TcError,
    ) -> u32;
    fn tc_guarded_json_h_s(
        func_ptr: *const c_void,
        handle: TcHandle,
        arg: *const c_char,
        out_ptr: *mut *const c_char,
    ) -> u32;
    fn tc_guarded_json_h(
        func_ptr: *const c_void,
        handle: TcHandle,
        out_ptr: *mut *const c_char,
    ) -> u32;
    fn tc_guarded_json_s(
        func_ptr: *const c_void,
        arg: *const c_char,
        out_ptr: *mut *const c_char,
    ) -> u32;
    fn tc_guarded_json_s_usize_usize(
        func_ptr: *const c_void,
        arg: *const c_char,
        a: usize,
        b: usize,
        out_ptr: *mut *const c_char,
    ) -> u32;
    fn tc_guarded_json_h_s_s(
        func_ptr: *const c_void,
        handle: TcHandle,
        a: *const c_char,
        b: *const c_char,
        out_ptr: *mut *const c_char,
    ) -> u32;
    fn tc_guarded_json_h_s_i32(
        func_ptr: *const c_void,
        handle: TcHandle,
        a: *const c_char,
        i32_arg: i32,
        out_ptr: *mut *const c_char,
    ) -> u32;
    fn tc_guarded_json_h_s_s_i32(
        func_ptr: *const c_void,
        handle: TcHandle,
        a: *const c_char,
        b: *const c_char,
        i32_arg: i32,
        out_ptr: *mut *const c_char,
    ) -> u32;
    fn tc_guarded_error_h(func_ptr: *const c_void, handle: TcHandle, out_code: *mut TcError)
    -> u32;
    fn tc_guarded_error_h_s(
        func_ptr: *const c_void,
        handle: TcHandle,
        a: *const c_char,
        out_code: *mut TcError,
    ) -> u32;
    fn tc_guarded_error_h_s_s(
        func_ptr: *const c_void,
        handle: TcHandle,
        a: *const c_char,
        b: *const c_char,
        out_code: *mut TcError,
    ) -> u32;
}

/// LSP 封装层的错误类型
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    /// 动态库加载失败
    #[error("ffi load failed: {0}")]
    FfiLoad(#[from] libloading::Error),
    /// JSON 序列化/反序列化失败
    #[error("json parse failed: {0}")]
    Json(#[from] serde_json::Error),
    /// UTF-8 转换失败
    #[error("utf8 parse failed: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    /// NulError
    #[error("nul error: {0}")]
    Nul(#[from] NulError),
    /// 内部错误
    #[error("internal error: {0}")]
    Internal(String),
}
