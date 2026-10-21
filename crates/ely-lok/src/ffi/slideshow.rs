use std::{ffi::c_char, ffi::c_void, io};

use super::{Doc, Document, cstring};

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

/// One rendered layer: the kernel's JSON and, for bitmaps, pixels.
pub struct Layer {
    pub json: String,
    pub pixels: Option<Vec<u8>>,
}

fn owned(raw: *mut c_char) -> io::Result<String> {
    if raw.is_null() {
        return Err(io::Error::other("no text from the kernel"));
    }
    let text = unsafe { std::ffi::CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { free(raw.cast()) };
    Ok(text)
}

impl Document {
    /// The slideshow's slides, transitions and animations as JSON.
    pub fn presentation_info(&self) -> io::Result<String> {
        let class = unsafe { (*self.doc).class };
        type F = unsafe extern "C" fn(*mut Doc) -> *mut c_char;
        owned(call!(class, 74, F, self.doc))
    }

    /// Slide `part`'s layers at about `pixels`; gives the size used.
    pub fn slide_layers(
        &self,
        (part, hash): (i32, &str),
        pixels: (u32, u32),
        max_bytes: usize,
    ) -> io::Result<((u32, u32), Vec<Layer>)> {
        let class = unsafe { (*self.doc).class };
        type Create = unsafe extern "C" fn(*mut Doc, *const c_char, i32, *mut u32, *mut u32, bool, bool) -> bool;
        type Next = unsafe extern "C" fn(*mut Doc, *mut u8, *mut bool, *mut f64, *mut *mut c_char) -> bool;
        type Cleanup = unsafe extern "C" fn(*mut Doc);
        let hash = cstring(hash)?;
        let (mut width, mut height) = pixels;
        let made = call!(class, 75, Create, self.doc, hash.as_ptr(), part, &mut width, &mut height, true, true);
        if !made {
            return Err(io::Error::other(format!("no slide renderer for slide {part}")));
        }
        let size = (width as usize) * (height as usize) * 4;
        let mut layers = Vec::new();
        let mut total = 0;
        let result = loop {
            let mut pixels = vec![0u8; size];
            let (mut bitmap, mut scale, mut json) = (false, 1.0f64, std::ptr::null_mut());
            let done = call!(class, 77, Next, self.doc, pixels.as_mut_ptr(), &mut bitmap, &mut scale, &mut json);
            if done {
                break Ok(());
            }
            let json = match owned(json) {
                Ok(json) => json,
                Err(error) => break Err(error),
            };
            total += json.len() + if bitmap { size } else { 0 };
            if total > max_bytes {
                break Err(io::Error::other(format!("slide {part}'s layers pass {max_bytes} bytes")));
            }
            layers.push(Layer { json, pixels: bitmap.then_some(pixels) });
        };
        call!(class, 76, Cleanup, self.doc);
        result.map(|()| ((width, height), layers))
    }
}
