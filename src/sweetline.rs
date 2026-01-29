#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use libc::intptr_t;
use std::ffi::{CStr, CString};
use std::slice;

/// 错误类型
#[derive(Debug)]
pub enum SweetLineError {
    JsonPropertyMissed,
    JsonPropertyInvalid,
    PatternInvalid,
    StateInvalid,
    JsonInvalid,
    FileIoError,
    FileEmpty,
    Unknown(i32),
    Message(String),
}

impl From<sl_error_t> for SweetLineError {
    fn from(code: sl_error_t) -> Self {
        match code {
            sl_error_SL_JSON_PROPERTY_MISSED => SweetLineError::JsonPropertyMissed,
            sl_error_SL_JSON_PROPERTY_INVALID => SweetLineError::JsonPropertyInvalid,
            sl_error_SL_PATTERN_INVALID => SweetLineError::PatternInvalid,
            sl_error_SL_STATE_INVALID => SweetLineError::StateInvalid,
            sl_error_SL_JSON_INVALID => SweetLineError::JsonInvalid,
            sl_error_SL_FILE_IO_ERR => SweetLineError::FileIoError,
            sl_error_SL_FILE_EMPTY => SweetLineError::FileEmpty,
            _ => SweetLineError::Unknown(code as i32),
        }
    }
}

pub struct Engine {
    handle: intptr_t,
}

impl Engine {
    pub fn new(show_index: bool) -> Self {
        unsafe {
            let handle = sl_create_engine(show_index);
            Self { handle }
        }
    }

    pub fn compile_json(&self, json: &str) -> Result<(), SweetLineError> {
        let c_json = CString::new(json).map_err(|_| SweetLineError::JsonInvalid)?;
        unsafe {
            let err = sl_engine_compile_json(self.handle, c_json.as_ptr());
            if err.err_code == sl_error_SL_OK {
                Ok(())
            } else {
                let msg = CStr::from_ptr(err.err_msg).to_string_lossy().into_owned();
                Err(SweetLineError::Message(msg))
            }
        }
    }

    pub fn load_document(&self, doc: &Document) -> DocumentAnalyzer {
        unsafe {
            let analyzer_handle = sl_engine_load_document(self.handle, doc.handle);
            DocumentAnalyzer {
                handle: analyzer_handle,
            }
        }
    }

    pub fn remove_document(&self, uri: &str) -> Result<(), SweetLineError> {
        let c_uri = CString::new(uri).map_err(|_| SweetLineError::JsonInvalid)?;
        unsafe {
            let err = sl_engine_remove_document(self.handle, c_uri.as_ptr());
            if err == sl_error_SL_OK {
                Ok(())
            } else {
                Err(SweetLineError::from(err))
            }
        }
    }

    pub fn get_style_name(&self, style_id: u32) -> Option<String> {
        unsafe {
            let ptr = sl_engine_get_style_name(self.handle, style_id as i32);
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            sl_free_engine(self.handle);
        }
    }
}

pub struct Document {
    handle: intptr_t,
}

impl Document {
    pub fn new(uri: &str, text: &str) -> Self {
        let c_uri = CString::new(uri).unwrap();
        let c_text = CString::new(text).unwrap();
        unsafe {
            let handle = sl_create_document(c_uri.as_ptr(), c_text.as_ptr());
            Self { handle }
        }
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        unsafe {
            sl_free_document(self.handle);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HighlightSpan {
    pub start_line: u32,
    pub start_col: u32,
    pub start_index: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub end_index: u32,
    pub style_id: u32,
    pub foreground: Option<u32>,
    pub background: Option<u32>,
    pub tags: Option<u32>,
}

pub struct DocumentAnalyzer {
    handle: intptr_t,
}

impl DocumentAnalyzer {
    pub fn parse_result(data: &[i32], inline_style: bool) -> Vec<HighlightSpan> {
        let stride = if inline_style { 10 } else { 7 };
        let mut spans = Vec::new();

        for chunk in data.chunks(stride) {
            if chunk.len() < stride {
                break;
            }

            let mut span = HighlightSpan {
                start_line: chunk[0] as u32,
                start_col: chunk[1] as u32,
                start_index: chunk[2] as u32,
                end_line: chunk[3] as u32,
                end_col: chunk[4] as u32,
                end_index: chunk[5] as u32,
                style_id: 0,
                foreground: None,
                background: None,
                tags: None,
            };

            if inline_style {
                span.foreground = Some(chunk[6] as u32);
                span.background = Some(chunk[7] as u32);
                span.tags = Some(chunk[8] as u32);
            } else {
                span.style_id = chunk[6] as u32;
            }

            spans.push(span);
        }

        spans
    }

    pub fn analyze(&self) -> Vec<i32> {
        unsafe {
            let mut size: i32 = 0;
            let ptr = sl_document_analyze(self.handle, &mut size);
            if ptr.is_null() || size == 0 {
                return Vec::new();
            }
            let slice = slice::from_raw_parts(ptr, size as usize);
            let result = slice.to_vec();
            sl_free_result(ptr);
            result
        }
    }

    pub fn analyze_incremental(
        &self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        new_text: &str,
    ) -> Vec<i32> {
        let mut range = [start_line, start_col, end_line, end_col];
        let c_text = CString::new(new_text).unwrap();
        unsafe {
            let mut size: i32 = 0;
            let ptr = sl_document_analyze_incremental(
                self.handle,
                range.as_mut_ptr(),
                c_text.as_ptr(),
                &mut size,
            );
            if ptr.is_null() || size == 0 {
                return Vec::new();
            }
            let slice = slice::from_raw_parts(ptr, size as usize);
            let result = slice.to_vec();
            sl_free_result(ptr);
            result
        }
    }
}
