pub mod tiec;

use libloading::Library;
use serde::de::DeserializeOwned;
use std::ffi::c_void;
use std::ffi::{CStr, CString, NulError};
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Mutex;

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
    /// 字符串包含 NUL 字节，无法传递给 C API
    #[error("nul byte in string: {0}")]
    Nul(#[from] NulError),
    /// C API 返回空指针
    #[error("ffi returned null pointer")]
    NullPtr,
    /// 返回了 0 句柄
    #[error("invalid handle: {0}")]
    InvalidHandle(&'static str),
    /// C API 返回的错误码
    #[error("ffi error code: {0}")]
    FfiError(TcError),
    #[error("options invalid: {0}")]
    Options(#[from] tiec::OptionsError),
    #[error("ffi crashed with seh exception: 0x{0:08X}")]
    SehException(u32),
}

/// UI 绑定获取结果的两种格式
pub enum UiBindings {
    /// TLY 文本格式
    Tly(String),
    /// JSON 结构格式
    Json(tiec::TlyEntity),
}

/// 动态库 LSP 服务封装
pub struct TiecLsp {
    /// 动态库加载句柄
    library: Library,
    /// 编译器上下文句柄
    context_handle: TcHandle,
    /// IDE 服务句柄
    ide_service_handle: TcHandle,
    pinned_file_lists: Mutex<Vec<PinnedFileList>>,
}

struct PinnedFileList {
    c_strings: Vec<*mut c_char>,
    raw_files: Box<[*const c_char]>,
}

impl TiecLsp {
    fn normalize_uri_for_json_arg(input: &str) -> String {
        let s = input.trim();
        let normalized_path = Self::normalize_compile_file_arg(s).replace('\\', "/");
        let mut encoded = String::with_capacity(normalized_path.len() + 16);
        for b in normalized_path.as_bytes() {
            let c = *b;
            let is_unreserved =
                matches!(c, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~');
            let is_allowed = is_unreserved || c == b'/' || c == b':';
            if is_allowed {
                encoded.push(c as char);
            } else {
                encoded.push('%');
                encoded.push_str(&format!("{:02X}", c));
            }
        }
        format!("file:///{}", encoded.trim_start_matches('/'))
    }

    pub fn new<P: AsRef<Path>>(dll_path: P, options: &tiec::Options) -> Result<Self, LspError> {
        options.validate()?;
        let library = unsafe { Library::new(dll_path.as_ref())? };
        let options_json = serde_json::to_string(options)?;
        let context_handle = Self::create_context(&library, &options_json)?;
        if context_handle == 0 {
            return Err(LspError::InvalidHandle("context_handle"));
        }
        let ide_service_handle = Self::create_ide_service(&library, context_handle)?;
        if ide_service_handle == 0 {
            return Err(LspError::InvalidHandle("ide_service_handle"));
        }
        Ok(Self {
            library,
            context_handle,
            ide_service_handle,
            pinned_file_lists: Mutex::new(Vec::new()),
        })
    }

    pub fn context_handle(&self) -> TcHandle {
        self.context_handle
    }

    pub fn ide_service_handle(&self) -> TcHandle {
        self.ide_service_handle
    }

    pub fn compile_files<S: AsRef<str>>(&self, files: &[S]) -> Result<(), LspError> {
        let c_files: Vec<CString> = files
            .iter()
            .map(|file| Self::cstring_for_ffi(&Self::normalize_compile_file_arg(file.as_ref())))
            .collect::<Result<_, _>>()?;

        let mut pinned = PinnedFileList {
            c_strings: Vec::with_capacity(c_files.len()),
            raw_files: Box::new([]),
        };

        for c in c_files {
            pinned.c_strings.push(c.into_raw());
        }

        let raw_files_vec: Vec<*const c_char> = pinned
            .c_strings
            .iter()
            .map(|p| *p as *const c_char)
            .collect();
        pinned.raw_files = raw_files_vec.into_boxed_slice();

        let func = *self
            .symbol::<unsafe extern "C" fn(TcHandle, usize, *const *const c_char) -> TcError>(
                b"tc_ide_service_compile_files\0",
            )?;

        #[cfg(windows)]
        let result = {
            let mut out_code: TcError = 0;
            let exception_code = unsafe {
                tc_guarded_ide_service_compile_files(
                    func as *const () as *const c_void,
                    self.ide_service_handle,
                    pinned.raw_files.len(),
                    pinned.raw_files.as_ptr(),
                    &mut out_code as *mut TcError,
                )
            };
            if exception_code != 0 {
                Self::free_pinned_file_list(pinned);
                return Err(LspError::SehException(exception_code));
            }
            out_code
        };

        #[cfg(not(windows))]
        let result = unsafe {
            func(
                self.ide_service_handle,
                pinned.raw_files.len(),
                pinned.raw_files.as_ptr(),
            )
        };

        if let Err(e) = Self::ensure_ok(result) {
            Self::free_pinned_file_list(pinned);
            return Err(e);
        }

        match self.pinned_file_lists.lock() {
            Ok(mut guard) => guard.push(pinned),
            Err(poisoned) => poisoned.into_inner().push(pinned),
        }
        Ok(())
    }

    fn free_pinned_file_list(pinned: PinnedFileList) {
        for p in pinned.c_strings {
            if !p.is_null() {
                unsafe {
                    drop(CString::from_raw(p));
                }
            }
        }
    }

    fn normalize_compile_file_arg(input: &str) -> String {
        let s = input.trim();
        if !s.starts_with("file:") {
            return s.to_string();
        }

        let mut rest = &s["file:".len()..];
        while rest.starts_with('/') {
            rest = &rest[1..];
        }

        let bytes = rest.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hi = bytes[i + 1];
                let lo = bytes[i + 2];
                let hex = |c: u8| -> Option<u8> {
                    match c {
                        b'0'..=b'9' => Some(c - b'0'),
                        b'a'..=b'f' => Some(c - b'a' + 10),
                        b'A'..=b'F' => Some(c - b'A' + 10),
                        _ => None,
                    }
                };
                if let (Some(h), Some(l)) = (hex(hi), hex(lo)) {
                    out.push((h << 4) | l);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }

        String::from_utf8_lossy(&out).into_owned()
    }

    pub fn edit_source(&self, uri: &str, new_text: &str) -> Result<(), LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let new_text = CString::new(new_text)?;
        let func = self
            .symbol::<unsafe extern "C" fn(TcHandle, *const c_char, *const c_char) -> TcError>(
                b"tc_ide_service_edit_source\0",
            )?;
        #[cfg(windows)]
        {
            let mut out_code: TcError = 0;
            let code = unsafe {
                tc_guarded_error_h_s_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    new_text.as_ptr(),
                    &mut out_code as *mut TcError,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            Self::ensure_ok(out_code)
        }
        #[cfg(not(windows))]
        {
            let result =
                unsafe { (*func)(self.ide_service_handle, uri.as_ptr(), new_text.as_ptr()) };
            Self::ensure_ok(result)
        }
    }

    pub fn edit_source_incremental(
        &self,
        uri: &str,
        change: &tiec::TextChange,
    ) -> Result<(), LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let change_json = CString::new(serde_json::to_string(change)?)?;
        let func = self
            .symbol::<unsafe extern "C" fn(TcHandle, *const c_char, *const c_char) -> TcError>(
                b"tc_ide_service_edit_source_incremental\0",
            )?;
        #[cfg(windows)]
        {
            let mut out_code: TcError = 0;
            let code = unsafe {
                tc_guarded_error_h_s_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    change_json.as_ptr(),
                    &mut out_code as *mut TcError,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            Self::ensure_ok(out_code)
        }
        #[cfg(not(windows))]
        {
            let result =
                unsafe { (*func)(self.ide_service_handle, uri.as_ptr(), change_json.as_ptr()) };
            Self::ensure_ok(result)
        }
    }

    pub fn create_source(&self, uri: &str, initial_text: &str) -> Result<(), LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let initial_text = CString::new(initial_text)?;
        let func = self
            .symbol::<unsafe extern "C" fn(TcHandle, *const c_char, *const c_char) -> TcError>(
                b"tc_ide_service_create_source\0",
            )?;
        #[cfg(windows)]
        {
            let mut out_code: TcError = 0;
            let code = unsafe {
                tc_guarded_error_h_s_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    initial_text.as_ptr(),
                    &mut out_code as *mut TcError,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            Self::ensure_ok(out_code)
        }
        #[cfg(not(windows))]
        {
            let result =
                unsafe { (*func)(self.ide_service_handle, uri.as_ptr(), initial_text.as_ptr()) };
            Self::ensure_ok(result)
        }
    }

    pub fn delete_source(&self, uri: &str) -> Result<(), LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> TcError>(
            b"tc_ide_service_delete_source\0",
        )?;
        #[cfg(windows)]
        {
            let mut out_code: TcError = 0;
            let code = unsafe {
                tc_guarded_error_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    &mut out_code as *mut TcError,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            Self::ensure_ok(out_code)
        }
        #[cfg(not(windows))]
        {
            let result = unsafe { (*func)(self.ide_service_handle, uri.as_ptr()) };
            Self::ensure_ok(result)
        }
    }

    pub fn rename_source(&self, uri: &str, new_uri: &str) -> Result<(), LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let new_uri = CString::new(Self::normalize_uri_for_json_arg(new_uri))?;
        let func = self
            .symbol::<unsafe extern "C" fn(TcHandle, *const c_char, *const c_char) -> TcError>(
                b"tc_ide_service_rename_source\0",
            )?;
        #[cfg(windows)]
        {
            let mut out_code: TcError = 0;
            let code = unsafe {
                tc_guarded_error_h_s_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    new_uri.as_ptr(),
                    &mut out_code as *mut TcError,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            Self::ensure_ok(out_code)
        }
        #[cfg(not(windows))]
        {
            let result =
                unsafe { (*func)(self.ide_service_handle, uri.as_ptr(), new_uri.as_ptr()) };
            Self::ensure_ok(result)
        }
    }

    pub fn complete(
        &self,
        params: &tiec::CompletionParams,
    ) -> Result<tiec::CompletionResult, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_complete\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn hover(&self, params: &tiec::CursorParams) -> Result<tiec::MarkupContent, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_hover\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn lint_file(&self, uri: &str) -> Result<tiec::LintResult, LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_lint_file\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, uri.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn lint_all(&self) -> Result<tiec::LintResult, LspError> {
        let func = self.symbol::<unsafe extern "C" fn(TcHandle) -> *const c_char>(
            b"tc_ide_service_lint_all\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle) };
        self.read_json(ptr)
    }

    pub fn highlight(&self, uri: &str) -> Result<tiec::HighlightResult, LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_highlight\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, uri.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn format(&self, uri: &str) -> Result<tiec::FormattingResult, LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_format\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, uri.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn source_elements(&self, uri: &str) -> Result<tiec::SourceElementsResult, LspError> {
        let uri = CString::new(Self::normalize_uri_for_json_arg(uri))?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_source_elements\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    uri.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, uri.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn workspace_elements(
        &self,
        keyword: &str,
    ) -> Result<tiec::WorkspaceElementsResult, LspError> {
        let keyword = CString::new(keyword)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_workspace_elements\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    keyword.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, keyword.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn signature_help(
        &self,
        params: &tiec::SignatureHelpParams,
    ) -> Result<tiec::SignatureHelpResult, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_signature_help\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn find_definition(
        &self,
        params: &tiec::CursorParams,
    ) -> Result<tiec::DefinitionResult, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_find_definition\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn find_references(
        &self,
        params: &tiec::CursorParams,
    ) -> Result<tiec::ReferenceResult, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_find_references\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn prepare_rename(
        &self,
        params: &tiec::CursorParams,
    ) -> Result<tiec::RenameSymbolInfo, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_prepare_rename\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn rename(
        &self,
        params: &tiec::CursorParams,
        new_name: &str,
    ) -> Result<tiec::RenameResult, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let new_name = CString::new(new_name)?;
        let func = self.symbol::<unsafe extern "C" fn(
            TcHandle,
            *const c_char,
            *const c_char,
        ) -> *const c_char>(b"tc_ide_service_rename\0")?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    new_name.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe {
            (*func)(
                self.ide_service_handle,
                params_json.as_ptr(),
                new_name.as_ptr(),
            )
        };
        self.read_json(ptr)
    }

    pub fn smart_enter(
        &self,
        params: &tiec::CursorParams,
    ) -> Result<tiec::SmartEnterResult, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_smart_enter\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn generate_event(
        &self,
        params: &tiec::CursorParams,
    ) -> Result<tiec::CodeActionResult, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_generate_event\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn support_ui_binding(
        &self,
        params: &tiec::CursorParams,
    ) -> Result<tiec::UIBindingSupportInfo, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_support_ui_binding\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn get_ui_bindings(
        &self,
        params: &tiec::CursorParams,
        format: tiec::TlyFormat,
    ) -> Result<UiBindings, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let func = self
            .symbol::<unsafe extern "C" fn(TcHandle, *const c_char, i32) -> *const c_char>(
                b"tc_ide_service_get_ui_bindings\0",
            )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s_i32(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    format as i32,
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, params_json.as_ptr(), format as i32) };
        let text = self.read_string(ptr)?;
        match format {
            tiec::TlyFormat::Json => Ok(UiBindings::Json(serde_json::from_str(&text)?)),
            tiec::TlyFormat::Tly => Ok(UiBindings::Tly(text)),
        }
    }

    pub fn parse_tly_entity(&self, tly_text: &str) -> Result<tiec::TlyParsingResult, LspError> {
        let tly_text = CString::new(tly_text)?;
        let func = self.symbol::<unsafe extern "C" fn(TcHandle, *const c_char) -> *const c_char>(
            b"tc_ide_service_parse_tly_entity\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    tly_text.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle, tly_text.as_ptr()) };
        self.read_json(ptr)
    }

    pub fn edit_ui_bindings(
        &self,
        params: &tiec::CursorParams,
        new_tly_data: &str,
        format: tiec::TlyFormat,
    ) -> Result<tiec::UIBindingEditResult, LspError> {
        let mut params = params.clone();
        params.uri = Self::normalize_uri_for_json_arg(&params.uri);
        let params_json = CString::new(serde_json::to_string(&params)?)?;
        let new_tly_data = CString::new(new_tly_data)?;
        let func = self.symbol::<unsafe extern "C" fn(
            TcHandle,
            *const c_char,
            *const c_char,
            i32,
        ) -> *const c_char>(b"tc_ide_service_edit_ui_bindings\0")?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h_s_s_i32(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    params_json.as_ptr(),
                    new_tly_data.as_ptr(),
                    format as i32,
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe {
            (*func)(
                self.ide_service_handle,
                params_json.as_ptr(),
                new_tly_data.as_ptr(),
                format as i32,
            )
        };
        self.read_json(ptr)
    }

    pub fn scan_ui_classes(&self) -> Result<tiec::ViewClassInfoResult, LspError> {
        let func = self.symbol::<unsafe extern "C" fn(TcHandle) -> *const c_char>(
            b"tc_ide_service_scan_ui_classes\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_h(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(self.ide_service_handle) };
        self.read_json(ptr)
    }

    pub fn cancel(&self) -> Result<(), LspError> {
        let func =
            self.symbol::<unsafe extern "C" fn(TcHandle) -> TcError>(b"tc_ide_service_cancel\0")?;
        #[cfg(windows)]
        {
            let mut out_code: TcError = 0;
            let code = unsafe {
                tc_guarded_error_h(
                    *func as *const () as *const c_void,
                    self.ide_service_handle,
                    &mut out_code as *mut TcError,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            Self::ensure_ok(out_code)
        }
        #[cfg(not(windows))]
        {
            let result = unsafe { (*func)(self.ide_service_handle) };
            Self::ensure_ok(result)
        }
    }

    pub fn format_text(&self, doc_text: &str) -> Result<String, LspError> {
        let doc_text = CString::new(doc_text)?;
        let func = self.symbol::<unsafe extern "C" fn(*const c_char) -> *const c_char>(
            b"tc_ide_service_format_text\0",
        )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_s(
                    *func as *const () as *const c_void,
                    doc_text.as_ptr(),
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(doc_text.as_ptr()) };
        self.read_string(ptr)
    }

    pub fn newline(&self, doc_text: &str, line: usize, column: usize) -> Result<String, LspError> {
        let doc_text = CString::new(doc_text)?;
        let func = self
            .symbol::<unsafe extern "C" fn(*const c_char, usize, usize) -> *const c_char>(
                b"tc_ide_service_newline\0",
            )?;
        #[cfg(windows)]
        let ptr = {
            let mut out: *const c_char = std::ptr::null();
            let code = unsafe {
                tc_guarded_json_s_usize_usize(
                    *func as *const () as *const c_void,
                    doc_text.as_ptr(),
                    line,
                    column,
                    &mut out as *mut *const c_char,
                )
            };
            if code != 0 {
                return Err(LspError::SehException(code));
            }
            out
        };
        #[cfg(not(windows))]
        let ptr = unsafe { (*func)(doc_text.as_ptr(), line, column) };
        self.read_string(ptr)
    }

    pub fn indent_advance(&self, line_text: &str, column: usize) -> Result<i32, LspError> {
        let line_text = CString::new(line_text)?;
        let func = self.symbol::<unsafe extern "C" fn(*const c_char, usize) -> i32>(
            b"tc_ide_service_indent_advance\0",
        )?;
        let result = unsafe { func(line_text.as_ptr(), column) };
        Ok(result)
    }

    fn create_context(library: &Library, options_json: &str) -> Result<TcHandle, LspError> {
        let options_json = CString::new(options_json)?;
        let func = unsafe {
            library
                .get::<unsafe extern "C" fn(*const c_char) -> TcHandle>(b"tc_create_context\0")?
        };
        Ok(unsafe { (*func)(options_json.as_ptr()) })
    }

    fn create_ide_service(
        library: &Library,
        context_handle: TcHandle,
    ) -> Result<TcHandle, LspError> {
        let func = unsafe {
            library.get::<unsafe extern "C" fn(TcHandle) -> TcHandle>(b"tc_create_ide_service\0")?
        };
        Ok(unsafe { (*func)(context_handle) })
    }

    fn symbol<T>(&self, name: &[u8]) -> Result<libloading::Symbol<'_, T>, LspError> {
        unsafe { self.library.get(name).map_err(LspError::from) }
    }

    fn read_string(&self, ptr: *const c_char) -> Result<String, LspError> {
        if ptr.is_null() {
            return Err(LspError::NullPtr);
        }
        let c_str = unsafe { CStr::from_ptr(ptr) };
        Ok(c_str.to_string_lossy().into_owned())
    }

    fn read_json<T: DeserializeOwned>(&self, ptr: *const c_char) -> Result<T, LspError> {
        let text = self.read_string(ptr)?;
        Ok(serde_json::from_str(&text)?)
    }

    fn cstring_for_ffi(s: &str) -> Result<CString, LspError> {
        Ok(CString::new(s)?)
    }

    fn ensure_ok(code: TcError) -> Result<(), LspError> {
        if code == 0 {
            Ok(())
        } else {
            Err(LspError::FfiError(code))
        }
    }
}

impl Drop for TiecLsp {
    fn drop(&mut self) {
        if self.ide_service_handle != 0 {
            if let Ok(func) =
                self.symbol::<unsafe extern "C" fn(TcHandle) -> TcError>(b"tc_free_ide_service\0")
            {
                let func = *func;
                #[cfg(windows)]
                unsafe {
                    let mut out_code: TcError = 0;
                    let _ = tc_guarded_free_handle(
                        func as *const () as *const c_void,
                        self.ide_service_handle,
                        &mut out_code as *mut TcError,
                    );
                }
                #[cfg(not(windows))]
                unsafe {
                    func(self.ide_service_handle);
                }
            }
            self.ide_service_handle = 0;
        }
        if self.context_handle != 0 {
            if let Ok(func) =
                self.symbol::<unsafe extern "C" fn(TcHandle) -> TcError>(b"tc_free_context\0")
            {
                let func = *func;
                #[cfg(windows)]
                unsafe {
                    let mut out_code: TcError = 0;
                    let _ = tc_guarded_free_handle(
                        func as *const () as *const c_void,
                        self.context_handle,
                        &mut out_code as *mut TcError,
                    );
                }
                #[cfg(not(windows))]
                unsafe {
                    func(self.context_handle);
                }
            }
            self.context_handle = 0;
        }

        if let Ok(mut guard) = self.pinned_file_lists.lock() {
            for pinned in guard.drain(..) {
                for p in pinned.c_strings {
                    if !p.is_null() {
                        unsafe {
                            drop(CString::from_raw(p));
                        }
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use crate::lsp::{
        TiecLsp,
        tiec::{CompletionParams, Options, Platform, Position},
    };

    /// 遍历指定路径下指定后缀的文件，返回完整路径列表
    fn find_files_with_extension<P: AsRef<Path>>(dir: P, extension: &str) -> Vec<String> {
        let mut result = Vec::new();
        let dir = dir.as_ref();

        if dir.is_dir() {
            // 遍历目录
            let entries = fs::read_dir(dir).unwrap_or_else(|_| panic!("无法读取目录: {:?}", dir));
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        // 递归子目录
                        result.extend(find_files_with_extension(&path, extension));
                    } else if path.is_file() {
                        // 检查后缀名
                        if let Some(ext) = path.extension() {
                            if ext == extension {
                                // 转换为完整路径字符串
                                if let Some(path_str) = path.to_str() {
                                    result.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }

    #[test]
    #[ignore]
    fn create_ide_server() {
        let dll_path = "C:/Users/xiaoa/Desktop/tie_rust_gpui/tiecode/tiec.dll";
        if !Path::new(dll_path).exists() {
            return;
        }

        let mut options = Options::ide();
        options.package_name = Some("结绳.中文".to_string());
        options.platform = Some(Platform::Windows);

        let dll = TiecLsp::new(dll_path, &options).unwrap();
        let files = find_files_with_extension("C:/Users/xiaoa/Desktop/结绳demo", "t");
        let _ = dll.compile_files(&files);
        println!("创建IDE服务器成功");

        let sample_path = "C:/Users/xiaoa/Desktop/结绳demo/sample.t";
        let mut sample_text = String::new();
        if let Ok(text) = fs::read_to_string(sample_path) {
            sample_text = text;
            let _ = dll.create_source(sample_path, &sample_text);
        }

        //语义高亮
        let uri = "file:///C:/Users/xiaoa/Desktop/结绳demo/sample.t";
        if let Ok(res) = dll.highlight(uri) {
            for h in res.highlights {
                let s = h.range.start;
                let e = h.range.end;
                println!(
                    "range={}:{}-{}:{}, kind={:?}, tags={:?}",
                    s.line, s.column, e.line, e.column, h.kind, h.tags
                );
            }
        } else {
            println!("highlight: no result");
        }

        let lines: Vec<&str> = sample_text.lines().collect();
        let safe_line = if lines.is_empty() {
            0
        } else {
            16.min(lines.len() - 1)
        };
        let safe_col = 0;
        let completion_params = CompletionParams {
            uri: sample_path.to_string(),
            position: Position {
                line: safe_line,
                column: safe_col,
            },
            line_text: lines.get(safe_line).map(|s| s.to_string()),
            partial: String::new(),
            trigger_char: None,
        };

        let json = serde_json::to_string_pretty(&completion_params).unwrap();
        println!("{}", json);

        match dll.complete(&completion_params) {
            Ok(res) => {
                for ele in res.items {
                    println!("{}", ele.label);
                }
            }
            Err(e) => {
                println!("complete: no result \n{}", e);
            }
        }
    }
}
