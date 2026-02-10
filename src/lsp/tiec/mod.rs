#![allow(dead_code)]

pub mod types;
pub mod wrapper;
#[cfg(test)]
mod test;

use libc::{c_char, intptr_t};
use libloading::Library;

pub type RawHandle = intptr_t;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcError {
    Ok = 0,
    HandleInvalid = 1,
    CompileFailed = 2,
    IoErr = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcTaskKind {
    Parse = 0,
    Enter,
    Attribute,
    Lower,
    Final
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcTlyFormat {
    Tly = 0,
    Json = 1,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcDeclarationKind {
    Java = 0,
    CppHeader = 1,
    Js = 2,
}

pub type TcSourceGetName = unsafe extern "C" fn() -> *const c_char;
pub type TcSourceLastModified = unsafe extern "C" fn() -> u64;
pub type TcSourceReadContent = unsafe extern "C" fn() -> *const c_char;
pub type TcSourceGetUri = unsafe extern "C" fn() -> *const c_char;
pub type TcSourceGetPath = unsafe extern "C" fn() -> *const c_char;

pub type TcTaskOnBegin = unsafe extern "C" fn(task_kind: TcTaskKind);
pub type TcTaskOnEnd = unsafe extern "C" fn(task_kind: TcTaskKind);

pub type TcDiagnosticReport = unsafe extern "C" fn(diagnostic_json: *const c_char);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TcSource {
    pub get_name: TcSourceGetName,
    pub last_modified: TcSourceLastModified,
    pub read_content: TcSourceReadContent,
    pub get_uri: TcSourceGetUri,
    pub get_path: TcSourceGetPath,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TcTaskListener {
    pub on_begin: TcTaskOnBegin,
    pub on_end: TcTaskOnEnd,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TcDiagnosticHandler {
    pub report: TcDiagnosticReport,
}

pub struct TiecLib {
    // We keep the library loaded
    _lib: Library,
    
    pub tc_create_context: unsafe extern "C" fn(options_json: *const c_char) -> RawHandle,
    pub tc_free_context: unsafe extern "C" fn(context_handle: RawHandle) -> TcError,
    pub tc_create_compiler: unsafe extern "C" fn(context_handle: RawHandle) -> RawHandle,
    pub tc_compiler_set_diagnostic_handler: unsafe extern "C" fn(compiler_handle: RawHandle, handler: TcDiagnosticHandler) -> TcError,
    pub tc_compiler_add_task_listener: unsafe extern "C" fn(compiler_handle: RawHandle, listener: TcTaskListener) -> TcError,
    pub tc_compiler_compile_files: unsafe extern "C" fn(compiler_handle: RawHandle, file_count: usize, files: *const *const c_char) -> TcError,
    pub tc_compiler_compile_sources: unsafe extern "C" fn(compiler_handle: RawHandle, source_count: usize, sources: *mut TcSource) -> TcError,
    pub tc_free_compiler: unsafe extern "C" fn(compiler_handle: RawHandle) -> TcError,
    pub tc_create_ide_service: unsafe extern "C" fn(context_handle: RawHandle) -> RawHandle,
    pub tc_ide_service_compile_files: unsafe extern "C" fn(ide_handle: RawHandle, file_count: usize, files: *const *const c_char) -> TcError,
    pub tc_ide_service_compile_sources: unsafe extern "C" fn(ide_handle: RawHandle, source_count: usize, sources: *mut TcSource) -> TcError,
    pub tc_ide_service_edit_source: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char, new_text: *const c_char) -> TcError,
    pub tc_ide_service_edit_source_incremental: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char, change_json: *const c_char) -> TcError,
    pub tc_ide_service_create_source: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char, initial_text: *const c_char) -> TcError,
    pub tc_ide_service_delete_source: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char) -> TcError,
    pub tc_ide_service_rename_source: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char, new_uri: *const c_char) -> TcError,
    pub tc_ide_service_complete: unsafe extern "C" fn(ide_handle: RawHandle, params_json: *const c_char) -> *const c_char,
    pub tc_ide_service_hover: unsafe extern "C" fn(ide_handle: RawHandle, params_json: *const c_char) -> *const c_char,
    pub tc_ide_service_lint_file: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char) -> *const c_char,
    pub tc_ide_service_lint_all: unsafe extern "C" fn(ide_handle: RawHandle) -> *const c_char,
    pub tc_ide_service_highlight: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char) -> *const c_char,
    pub tc_ide_service_format: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char) -> *const c_char,
    pub tc_ide_service_source_elements: unsafe extern "C" fn(ide_handle: RawHandle, uri: *const c_char) -> *const c_char,
    pub tc_ide_service_workspace_elements: unsafe extern "C" fn(ide_handle: RawHandle, keyword: *const c_char) -> *const c_char,
    pub tc_ide_service_format_text: unsafe extern "C" fn(doc_text: *const c_char) -> *const c_char,
    pub tc_ide_service_newline: unsafe extern "C" fn(doc_text: *const c_char, line: usize, column: usize) -> *const c_char,
    pub tc_ide_service_indent_advance: unsafe extern "C" fn(line_text: *const c_char, column: usize) -> i32,
}

impl TiecLib {
    pub unsafe fn load(path: &str) -> Result<Self, libloading::Error> {
        let lib = Library::new(path)?;
        
        // Helper macro to load symbols
        macro_rules! load_sym {
            ($name:literal) => {
                *lib.get($name)?
            }
        }

        Ok(Self {
            tc_create_context: load_sym!(b"tc_create_context"),
            tc_free_context: load_sym!(b"tc_free_context"),
            tc_create_compiler: load_sym!(b"tc_create_compiler"),
            tc_compiler_set_diagnostic_handler: load_sym!(b"tc_compiler_set_diagnostic_handler"),
            tc_compiler_add_task_listener: load_sym!(b"tc_compiler_add_task_listener"),
            tc_compiler_compile_files: load_sym!(b"tc_compiler_compile_files"),
            tc_compiler_compile_sources: load_sym!(b"tc_compiler_compile_sources"),
            tc_free_compiler: load_sym!(b"tc_free_compiler"),
            tc_create_ide_service: load_sym!(b"tc_create_ide_service"),
            tc_ide_service_compile_files: load_sym!(b"tc_ide_service_compile_files"),
            tc_ide_service_compile_sources: load_sym!(b"tc_ide_service_compile_sources"),
            tc_ide_service_edit_source: load_sym!(b"tc_ide_service_edit_source"),
            tc_ide_service_edit_source_incremental: load_sym!(b"tc_ide_service_edit_source_incremental"),
            tc_ide_service_create_source: load_sym!(b"tc_ide_service_create_source"),
            tc_ide_service_delete_source: load_sym!(b"tc_ide_service_delete_source"),
            tc_ide_service_rename_source: load_sym!(b"tc_ide_service_rename_source"),
            tc_ide_service_complete: load_sym!(b"tc_ide_service_complete"),
            tc_ide_service_hover: load_sym!(b"tc_ide_service_hover"),
            tc_ide_service_lint_file: load_sym!(b"tc_ide_service_lint_file"),
            tc_ide_service_lint_all: load_sym!(b"tc_ide_service_lint_all"),
            tc_ide_service_highlight: load_sym!(b"tc_ide_service_highlight"),
            tc_ide_service_format: load_sym!(b"tc_ide_service_format"),
            tc_ide_service_source_elements: load_sym!(b"tc_ide_service_source_elements"),
            tc_ide_service_workspace_elements: load_sym!(b"tc_ide_service_workspace_elements"),
            tc_ide_service_format_text: load_sym!(b"tc_ide_service_format_text"),
            tc_ide_service_newline: load_sym!(b"tc_ide_service_newline"),
            tc_ide_service_indent_advance: load_sym!(b"tc_ide_service_indent_advance"),
            _lib: lib,
        })
    }
}
