use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path, ptr};
use windows_sys::Win32::{
    Foundation::{CloseHandle, LocalFree},
    Security::{
        Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
            ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW,
            SE_FILE_OBJECT, SetNamedSecurityInfoW,
        },
        DACL_SECURITY_INFORMATION, GetSecurityDescriptorControl, GetSecurityDescriptorDacl,
        GetTokenInformation, PROTECTED_DACL_SECURITY_INFORMATION, SE_DACL_PROTECTED, TOKEN_QUERY,
        TOKEN_USER, TokenUser, UNPROTECTED_DACL_SECURITY_INFORMATION,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

struct Local(*mut c_void);

impl Drop for Local {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

unsafe fn string(raw: *const u16) -> String {
    let mut len = 0;
    unsafe {
        while *raw.add(len) != 0 {
            len += 1;
        }
        String::from_utf16(std::slice::from_raw_parts(raw, len)).expect("valid SDDL UTF-16")
    }
}

pub fn user_sid() -> String {
    unsafe {
        let mut token = ptr::null_mut();
        assert_ne!(
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token),
            0
        );
        let mut needed = 0;
        GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut needed);
        assert!(needed > 0);
        let mut buffer = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
        let result = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        );
        CloseHandle(token);
        assert_ne!(result, 0);
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let mut raw = ptr::null_mut();
        assert_ne!(ConvertSidToStringSidW(user.User.Sid, &mut raw), 0);
        let _allocation = Local(raw.cast());
        string(raw)
    }
}

pub fn set_dacl(path: &Path, sddl: &str) {
    let sddl: Vec<_> = sddl.encode_utf16().chain(Some(0)).collect();
    let mut path = wide(path);
    unsafe {
        let mut raw = ptr::null_mut();
        assert_ne!(
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut raw,
                ptr::null_mut()
            ),
            0
        );
        let _allocation = Local(raw);
        let (mut present, mut defaulted, mut acl) = (0, 0, ptr::null_mut());
        assert_ne!(
            GetSecurityDescriptorDacl(raw, &mut present, &mut acl, &mut defaulted),
            0
        );
        assert_ne!(present, 0);
        let (mut control, mut revision) = (0, 0);
        assert_ne!(
            GetSecurityDescriptorControl(raw, &mut control, &mut revision),
            0
        );
        let inheritance = if control & SE_DACL_PROTECTED != 0 {
            PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        };
        let result = SetNamedSecurityInfoW(
            path.as_mut_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | inheritance,
            ptr::null_mut(),
            ptr::null_mut(),
            acl,
            ptr::null_mut(),
        );
        assert_eq!(
            result,
            0,
            "set fixture DACL: {}",
            std::io::Error::from_raw_os_error(result as i32)
        );
    }
}

pub fn dacl(path: &Path) -> String {
    let path = wide(path);
    unsafe {
        let mut raw = ptr::null_mut();
        let result = GetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut raw,
        );
        assert_eq!(result, 0, "read fixture DACL");
        let _descriptor = Local(raw);
        let mut text = ptr::null_mut();
        assert_ne!(
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                raw,
                1,
                DACL_SECURITY_INFORMATION,
                &mut text,
                ptr::null_mut()
            ),
            0
        );
        let _text = Local(text.cast());
        string(text)
    }
}
