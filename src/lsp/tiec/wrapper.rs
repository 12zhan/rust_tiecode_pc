use std::ffi::{CStr, CString};
use std::sync::Arc;
use anyhow::{Result, anyhow};
use libc::c_char;
use log::{debug, info};
use super::{TiecLib, RawHandle, TcError};
use super::types::*;

pub struct TiecLoader {
    lib: Arc<TiecLib>,
}

impl TiecLoader {
    pub unsafe fn new(path: &str) -> Result<Self> {
        let lib = TiecLib::load(path)?;
        Ok(Self { lib: Arc::new(lib) })
    }

    pub fn create_context(&self, options: &serde_json::Value) -> Result<TiecContext> {
        let json = serde_json::to_string(options)?;
        let c_json = CString::new(json)?;
        
        let handle = microseh::try_seh(|| unsafe { 
            (self.lib.tc_create_context)(c_json.as_ptr()) 
        }).map_err(|e| anyhow!("create_context caused access violation: {:?}", e))?;

        if handle == 0 {
            return Err(anyhow!("Failed to create context"));
        }

        Ok(TiecContext {
            lib: self.lib.clone(),
            handle,
        })
    }
}

pub struct TiecContext {
    lib: Arc<TiecLib>,
    handle: RawHandle,
}

impl Drop for TiecContext {
    fn drop(&mut self) {
        // We can't easily return Result from drop, but we can catch panic to avoid crashing
        let _ = microseh::try_seh(|| unsafe {
            (self.lib.tc_free_context)(self.handle);
        });
    }
}

impl TiecContext {
    pub fn create_ide_service(self) -> Result<TiecIdeService> {
        let handle = microseh::try_seh(|| unsafe { 
            (self.lib.tc_create_ide_service)(self.handle) 
        }).map_err(|e| anyhow!("create_ide_service caused access violation: {:?}", e))?;

        if handle == 0 {
            return Err(anyhow!("Failed to create IDE service"));
        }
        
        // Note: The context must stay alive as long as the service is used?
        // The C API docs say: "Destroy compiler context... after using compiler OR IDE service"
        // This implies context owns the shared state.
        // We move 'self' into TiecIdeService to ensure Context is kept alive and dropped only when Service is dropped.
        // Wait, if we want to create multiple things from context (e.g. Compiler AND Service), we might need Arc.
        // For now, let's assume exclusive ownership or wrap in Arc if needed.
        // Given the C++ shared_ptr nature, maybe it's fine.
        // But to be safe in Rust wrapper, let's keep Context inside Service.
        
        Ok(TiecIdeService {
            lib: self.lib.clone(),
            handle,
            _context: Arc::new(self),
        })
    }
    
    // If we want to support sharing context, we'd need `create_ide_service(&self)` and return a struct that holds Arc<TiecContext>.
    // Let's change design slightly to allow sharing if needed, but for now specific use case implies 1 context -> 1 service usually.
}

pub struct TiecIdeService {
    lib: Arc<TiecLib>,
    handle: RawHandle,
    _context: Arc<TiecContext>,
}

impl TiecIdeService {
    fn call_json_op<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self, 
        op: unsafe extern "C" fn(RawHandle, *const c_char) -> *const c_char, 
        op_name: &str,
        params: &T
    ) -> Result<R> {
        let json = serde_json::to_string(params)?;
        debug!("Calling {} with handle {:?} and json: {}", op_name, self.handle, json);
        
        let c_json = CString::new(json)?;
        
        let result_ptr = unsafe { op(self.handle, c_json.as_ptr()) };
        debug!("{} returned ptr: {:?}", op_name, result_ptr);
        if result_ptr.is_null() {
            return Err(anyhow!("{} returned null", op_name));
        }
        
        let result_str = unsafe { CStr::from_ptr(result_ptr).to_str()? };
        debug!("{} result: {}", op_name, result_str);
        let result: R = serde_json::from_str(result_str)?;
        
        Ok(result)
    }

    fn call_void_op<T: serde::Serialize>(
        &self, 
        op: unsafe extern "C" fn(RawHandle, *const c_char) -> TcError, 
        op_name: &str,
        params: &T
    ) -> Result<()> {
        let json = serde_json::to_string(params)?;
        debug!("Calling {} with handle {:?} and json: {}", op_name, self.handle, json);
        
        let c_json = CString::new(json)?;
        
        let err = unsafe { op(self.handle, c_json.as_ptr()) };
        if err != TcError::Ok {
            return Err(anyhow!("{} failed with error: {:?}", op_name, err));
        }
        debug!("{} success", op_name);
        Ok(())
    }
    
    // --- API Methods ---

    pub fn compile_files(&self, files: &[String]) -> Result<()> {
        debug!("compile_files called with {} files", files.len());
        let c_files: Vec<CString> = files.iter()
            .map(|s| CString::new(s.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
            
        let c_ptrs: Vec<*const c_char> = c_files.iter()
            .map(|s| s.as_ptr())
            .collect();
            
        let err = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_compile_files)(
                self.handle, 
                c_ptrs.len(), 
                c_ptrs.as_ptr()
            ) 
        }).map_err(|e| anyhow!("compile_files caused access violation: {:?}", e))?;
        
        if err != TcError::Ok {
            return Err(anyhow!("compile_files failed: {:?}", err));
        }
        debug!("compile_files success");
        Ok(())
    }

    pub fn edit_source(&self, uri: &str, new_text: &str) -> Result<()> {
        debug!("edit_source: {}", uri);
        info!("Debug edit_source: uri={}, text_len={}", uri, new_text.len());
        
        let c_uri = CString::new(uri)?;
        let c_text = CString::new(new_text)?;
        let err = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_edit_source)(self.handle, c_uri.as_ptr(), c_text.as_ptr()) 
        }).map_err(|e| anyhow!("edit_source caused access violation: {:?}", e))?;

        if err != TcError::Ok {
            return Err(anyhow!("edit_source failed: {:?}", err));
        }
        Ok(())
    }

    pub fn edit_source_incremental(&self, uri: &str, change: &TextChange) -> Result<()> {
        debug!("edit_source_incremental: {}", uri);
        let json = serde_json::to_string(change)?;
        info!("Debug JSON for edit_source_incremental: uri={}, json={}", uri, json);
        
        let c_uri = CString::new(uri)?;
        let c_json = CString::new(json)?;
        
        let err = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_edit_source_incremental)(self.handle, c_uri.as_ptr(), c_json.as_ptr()) 
        }).map_err(|e| anyhow!("edit_source_incremental caused access violation: {:?}", e))?;

        if err != TcError::Ok {
            return Err(anyhow!("edit_source_incremental failed: {:?}", err));
        }
        Ok(())
    }
    
    pub fn create_source(&self, uri: &str, initial_text: &str) -> Result<()> {
        debug!("create_source: {}", uri);
        let c_uri = CString::new(uri)?;
        let c_text = CString::new(initial_text)?;
        let err = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_create_source)(self.handle, c_uri.as_ptr(), c_text.as_ptr()) 
        }).map_err(|e| anyhow!("create_source caused access violation: {:?}", e))?;

        if err != TcError::Ok {
            return Err(anyhow!("create_source failed: {:?}", err));
        }
        Ok(())
    }

    pub fn delete_source(&self, uri: &str) -> Result<()> {
        debug!("delete_source: {}", uri);
        let c_uri = CString::new(uri)?;
        let err = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_delete_source)(self.handle, c_uri.as_ptr()) 
        }).map_err(|e| anyhow!("delete_source caused access violation: {:?}", e))?;

        if err != TcError::Ok {
            return Err(anyhow!("delete_source failed: {:?}", err));
        }
        Ok(())
    }
    
    pub fn rename_source(&self, uri: &str, new_uri: &str) -> Result<()> {
        debug!("rename_source: {} -> {}", uri, new_uri);
        let c_uri = CString::new(uri)?;
        let c_new_uri = CString::new(new_uri)?;
        let err = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_rename_source)(self.handle, c_uri.as_ptr(), c_new_uri.as_ptr()) 
        }).map_err(|e| anyhow!("rename_source caused access violation: {:?}", e))?;

        if err != TcError::Ok {
            return Err(anyhow!("rename_source failed: {:?}", err));
        }
        Ok(())
    }

    pub fn complete(&self, params: &serde_json::Value) -> Result<CompletionResult> {
        let json = serde_json::to_string(params)?;
        debug!("Calling complete with handle {:?} and json: {}", self.handle, json);
        info!("Debug JSON for complete: {}", json);
        
        let c_json = CString::new(json)?;
        
        info!("Invoking FFI complete...");
        
        let result_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_complete)(self.handle, c_json.as_ptr()) 
        }).map_err(|e| anyhow!("complete caused access violation: {:?}", e))?;

        info!("FFI complete returned ptr: {:?}", result_ptr);
        
        if result_ptr.is_null() {
            return Err(anyhow!("complete returned null"));
        }
        
        let result_str = unsafe { CStr::from_ptr(result_ptr).to_str()? };
        debug!("complete result: {}", result_str);
        let result: CompletionResult = serde_json::from_str(result_str)?;
        
        Ok(result)
    }

    pub fn hover(&self, params: &CursorParams) -> Result<HoverResult> {
        let json = serde_json::to_string(params)?;
        let c_json = CString::new(json)?;
        
        let res_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_hover)(self.handle, c_json.as_ptr()) 
        }).map_err(|e| anyhow!("hover caused access violation: {:?}", e))?;

        if res_ptr.is_null() {
            return Err(anyhow!("hover returned null"));
        }
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str()? };
        Ok(serde_json::from_str(res_str)?)
    }
    
    pub fn lint_file(&self, uri: &str) -> Result<LintResult> {
        let c_uri = CString::new(uri)?;
        
        let res_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_lint_file)(self.handle, c_uri.as_ptr()) 
        }).map_err(|e| anyhow!("lint_file caused access violation: {:?}", e))?;

        if res_ptr.is_null() {
            return Err(anyhow!("lint_file returned null"));
        }
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str()? };
        Ok(serde_json::from_str(res_str)?)
    }
    
    pub fn lint_all(&self) -> Result<LintResult> {
        let res_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_lint_all)(self.handle) 
        }).map_err(|e| anyhow!("lint_all caused access violation: {:?}", e))?;

        if res_ptr.is_null() {
            return Err(anyhow!("lint_all returned null"));
        }
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str()? };
        Ok(serde_json::from_str(res_str)?)
    }

    pub fn highlight(&self, uri: &str) -> Result<HighlightResult> {
        let c_uri = CString::new(uri)?;
        
        let res_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_highlight)(self.handle, c_uri.as_ptr()) 
        }).map_err(|e| anyhow!("highlight caused access violation: {:?}", e))?;

        if res_ptr.is_null() {
            return Err(anyhow!("highlight returned null"));
        }
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str()? };
        Ok(serde_json::from_str(res_str)?)
    }
    
    pub fn format(&self, uri: &str) -> Result<serde_json::Value> {
        // Return Value as specific struct is not fully defined in docs yet
        let c_uri = CString::new(uri)?;
        
        let res_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_format)(self.handle, c_uri.as_ptr()) 
        }).map_err(|e| anyhow!("format caused access violation: {:?}", e))?;

        if res_ptr.is_null() {
            return Err(anyhow!("format returned null"));
        }
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str()? };
        Ok(serde_json::from_str(res_str)?)
    }
    
    pub fn source_elements(&self, uri: &str) -> Result<SourceElementsResult> {
        let c_uri = CString::new(uri)?;
        
        let res_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_source_elements)(self.handle, c_uri.as_ptr()) 
        }).map_err(|e| anyhow!("source_elements caused access violation: {:?}", e))?;

        if res_ptr.is_null() {
            return Err(anyhow!("source_elements returned null"));
        }
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str()? };
        Ok(serde_json::from_str(res_str)?)
    }
    
    // Static utility methods that don't need service handle but are part of lib
    
    pub fn format_text(&self, doc_text: &str) -> Result<String> {
        let c_text = CString::new(doc_text)?;
        
        let res_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_format_text)(c_text.as_ptr()) 
        }).map_err(|e| anyhow!("format_text caused access violation: {:?}", e))?;

        if res_ptr.is_null() {
            return Err(anyhow!("format_text returned null"));
        }
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str()? };
        Ok(res_str.to_string())
    }
    
    pub fn newline(&self, doc_text: &str, line: usize, column: usize) -> Result<String> {
        let c_text = CString::new(doc_text)?;
        
        let res_ptr = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_newline)(c_text.as_ptr(), line, column) 
        }).map_err(|e| anyhow!("newline caused access violation: {:?}", e))?;

        if res_ptr.is_null() {
            return Err(anyhow!("newline returned null"));
        }
        let res_str = unsafe { CStr::from_ptr(res_ptr).to_str()? };
        Ok(res_str.to_string())
    }
    
    pub fn indent_advance(&self, line_text: &str, column: usize) -> Result<i32> {
        let c_text = CString::new(line_text)?;
        
        let result = microseh::try_seh(|| unsafe { 
            (self.lib.tc_ide_service_indent_advance)(c_text.as_ptr(), column) 
        }).map_err(|e| anyhow!("indent_advance caused access violation: {:?}", e))?;

        Ok(result)
    }
}
