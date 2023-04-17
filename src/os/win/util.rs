use std::ffi::OsStr;
use std::path::PathBuf;

use raw_window_handle::HasRawWindowHandle;
use simple_error::SimpleResult as Result;

use crate::error::*;
use crate::os::shared::ActiveWindowInfo;

use winapi::ctypes::c_void;
use winapi::um::stringapiset::MultiByteToWideChar;
use winapi::um::winuser::{GetWindowTextW, MessageBoxW};
use winapi::um::processthreadsapi::{OpenProcess, GetCurrentProcess, TerminateProcess};

pub fn get_hwnd(window: &winit::window::Window) -> winapi::shared::windef::HWND {
    match window.raw_window_handle() {
        raw_window_handle::RawWindowHandle::Win32(wnd_handle) => {
            wnd_handle.hwnd as winapi::shared::windef::HWND
        }
        _ => panic!("No MS Windows specific window handle. Wrong platform?"),
    }
}

pub fn str_to_wide(string: &str) -> Vec<u16> {
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(string).encode_wide().chain(once(0)).collect()
}

pub fn wide_to_str(buf: &Vec<u16>) -> String {
    String::from_utf16_lossy(&buf)
}

pub fn wide_slice_to_str(buf: &[u16]) -> String {
    String::from_utf16_lossy(&buf)
}

/// From the given buffer `src_string` use the Windows API to convert the
/// ANSI string with the given `code_page`.
///
/// [MSDN Info](https://docs.microsoft.com/en-us/windows/win32/intl/code-pages)
pub fn ansi_str_to_wide(src_string: &Vec<i8>, code_page: u16) -> Result<Vec<u16>> {
    let receiving_capacity = src_string.len() * 2;
    // generally single byte strings use 1 byte per character
    // we allocate twice of that hoping we won't get truncated

    let mut dst_string = Vec::<u16>::with_capacity(receiving_capacity);
    let result = unsafe {
        MultiByteToWideChar(
            code_page as u32,
            0,
            src_string.as_ptr() as *const i8,
            src_string.len() as i32, // size in bytes is the same as the length
            dst_string.as_mut_ptr(),
            (receiving_capacity - 1) as i32,
        )
    };
    // this call is safe as long as we trust WinAPI
    // to respect the indicated capacities in cbMultiByte (param 4)
    // and in cchWideChar (param 6)

    if result == 0 {
        bail!("Could not convert the given string. Call GetLastError from WinAPI to find out why.");
    }

    Ok(dst_string)
}

pub fn as_u8_slice(v: &[u32]) -> &[u8] {
    let element_size = std::mem::size_of::<u32>();
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * element_size) }
}

pub fn get_exe_file_icon(path: &str) -> Result<winapi::shared::windef::HICON> {
    use winapi::um::shellapi::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};

    let wide_path = crate::os::str_to_wide(&path);
    let mut file_info: SHFILEINFOW = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let res = unsafe {
        SHGetFileInfoW(
            wide_path.as_ptr(),
            0,
            &mut file_info,
            std::mem::size_of_val(&file_info) as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };

    if res == 0 {
        bail!("Cannot get icon with SHGetFileInfoW for {}", path);
    }

    // Icons given by this function should be destroyed
    // with DestroyIcon. At the same time, according to this
    // https://docs.microsoft.com/en-gb/windows/win32/api/winuser/nf-winuser-loadimagea?redirectedfrom=MSDN#remarks
    // it looks like resources are automatically released
    // when the program ends which is what we need here
    // ToDO: investigate if this causes any memory leaks
    Ok(file_info.hIcon)
}

pub fn _get_config_directory() -> BSResult<String> {
    use winapi::shared::winerror::S_OK;
    use winapi::um::combaseapi::CoTaskMemFree;
    use winapi::um::knownfolders::FOLDERID_RoamingAppData;
    use winapi::um::shlobj::SHGetKnownFolderPath;

    let mut wide_system_path: *mut u16 = std::ptr::null_mut();
    let result_path: BSResult<String> = unsafe {
        match SHGetKnownFolderPath(
            &FOLDERID_RoamingAppData,
            0,
            std::ptr::null_mut(),
            &mut wide_system_path,
        ) {
            S_OK => {
                //let string_length = 0;
                let mut buff = Vec::<u16>::new();
                for i in 0usize.. {
                    if *wide_system_path.offset(i as isize) == 0 {
                        buff = Vec::<u16>::from_raw_parts(wide_system_path, i, i);
                    }
                }

                let path = wide_to_str(&buff);

                std::mem::forget(buff); // from_raw_parts does not clone the buffer
                                        // we need to allow the buffer to exist after the vec is dealocated
                                        // as we free the memory using CoTaskMemFree as recommended by the WinAPI

                Ok(path)
            }
            code => Err(BSError::from(
                format!("Error getting OS config directory. Error code: {:?}", code).as_str(),
            )),
        }
    };

    unsafe { CoTaskMemFree(wide_system_path as *mut c_void) };
    result_path
}

pub fn _get_create_config_directory(app_name: &str, env_name: &str) -> BSResult<String> {
    let app_data_dir = _get_config_directory()?;
    let os_path = std::path::Path::new(&app_data_dir);
    let subpath = format!("{}/{}", app_name, env_name);
    let app_env_path = std::path::Path::new(&subpath);
    let full_path = os_path.join(app_env_path);
    let full_path_str = full_path.to_string_lossy().to_string();

    if std::fs::create_dir_all(&full_path).is_err() {
        bail!("Error creating config path {}", full_path_str);
    }

    Ok(full_path_str)
}

pub fn output_panic_text(text: String) {
    let wide_text = str_to_wide(&text);
    let title = str_to_wide(&"Panic!");
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide_text.as_ptr(),
            title.as_ptr(),
            winapi::um::winuser::MB_OK,
        );
    }
}

pub fn get_active_window_info() -> ActiveWindowInfo {
    let mut window_name: Option<String> = None;
    let mut exe_path: Option<PathBuf> = None;
    unsafe {
        let fg_hwnd = winapi::um::winuser::GetForegroundWindow();
        if !fg_hwnd.is_null() {
            let mut process_id: u32 = 0;
            let thread_id = winapi::um::winuser::GetWindowThreadProcessId(
                fg_hwnd,
                std::ptr::addr_of_mut!(process_id),
            );
            if thread_id != 0 {
                let mut buffer: [u16; 1024] = [0 as u16; 1024];
                let mut buffer_capacity: u32 = 1023;
                let text_len = GetWindowTextW(fg_hwnd, buffer.as_mut_ptr(), buffer_capacity as i32);
                if text_len > 0 {
                    let sub_slice = &buffer[..text_len as usize];
                    window_name = Some(wide_slice_to_str(sub_slice));
                }

                let p_handle =
                    OpenProcess(winapi::um::winnt::PROCESS_QUERY_INFORMATION, 0, process_id);
                let success = winapi::um::winbase::QueryFullProcessImageNameW(
                    p_handle,
                    0,
                    buffer.as_mut_ptr(),
                    std::ptr::addr_of_mut!(buffer_capacity),
                );
                if success != 0 {
                    let sub_slice = &buffer[..buffer_capacity as usize];
                    exe_path = Some(PathBuf::from(wide_slice_to_str(sub_slice)));
                }
            }
        }
    }

    ActiveWindowInfo {
        window_name,
        exe_path,
    }
}

pub fn terminate_current_process() {
    unsafe {
        TerminateProcess(GetCurrentProcess(), 0);
    }
}
