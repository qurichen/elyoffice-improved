use std::{
    ffi::c_void,
    fs::File,
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::Path,
    ptr,
};
use windows_sys::Win32::{
    Foundation::{INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT, SetSecurityInfo},
        DACL_SECURITY_INFORMATION, GetSecurityDescriptorControl,
        PROTECTED_DACL_SECURITY_INFORMATION, SE_DACL_PROTECTED, SECURITY_ATTRIBUTES,
        SetSecurityDescriptorControl, UNPROTECTED_DACL_SECURITY_INFORMATION,
    },
    Storage::FileSystem::{
        CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_TEMPORARY,
        FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, WRITE_DAC,
    },
};

struct Descriptor(*mut c_void);

impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut units: Vec<_> = path.as_os_str().encode_wide().collect();
    if units.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the path holds a NUL",
        ));
    }
    units.push(0);
    Ok(units)
}

pub(super) fn create_private(from: &Path, to: &Path, temporary: bool) -> io::Result<File> {
    wide(from)?;
    wide(to)?;
    let from = wide(&super::resolved(from)?)?;
    let to_wide = wide(&super::resolved(to)?)?;
    unsafe {
        let (mut descriptor, mut acl) = (ptr::null_mut(), ptr::null_mut());
        let result = GetNamedSecurityInfoW(
            from.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut descriptor,
        );
        if result != 0 {
            return Err(io::Error::from_raw_os_error(result as i32));
        }
        let descriptor = Descriptor(descriptor);
        let (mut control, mut revision) = (0, 0);
        if GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) == 0 {
            return Err(io::Error::last_os_error());
        }
        let inheritance = if control & SE_DACL_PROTECTED != 0 {
            PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        };
        if SetSecurityDescriptorControl(descriptor.0, SE_DACL_PROTECTED, SE_DACL_PROTECTED) == 0 {
            return Err(io::Error::last_os_error());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        let flags = if temporary {
            FILE_ATTRIBUTE_TEMPORARY
        } else {
            FILE_ATTRIBUTE_NORMAL
        };
        let handle = CreateFileW(
            to_wide.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | WRITE_DAC,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &attributes,
            CREATE_NEW,
            flags,
            ptr::null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let file = File::from_raw_handle(handle);
        let result = SetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | inheritance,
            ptr::null_mut(),
            ptr::null_mut(),
            acl,
            ptr::null_mut(),
        );
        if result != 0 {
            let error = io::Error::from_raw_os_error(result as i32);
            drop(file);
            return match std::fs::remove_file(to) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(io::Error::new(
                    error.kind(),
                    format!("{error}; removing empty staging file failed: {cleanup}"),
                )),
            };
        }
        Ok(file)
    }
}

#[cfg(test)]
mod tests;
