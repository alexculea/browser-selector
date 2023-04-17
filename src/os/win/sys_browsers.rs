use crate::{error::BSResult as Result, ui::{BrowserSelectorUI, UserInterface, ListItem}};
mod winapi {
    pub use winapi::shared::minwindef::DWORD;
    pub use winapi::shared::windef::HICON;
    pub use winapi::um::errhandlingapi::GetLastError;
    pub use winapi::um::winbase::GetBinaryTypeW;
    pub use winapi::um::winnls::GetUserDefaultUILanguage;
    pub use winapi::um::winver::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    pub use winapi::ctypes::*;
}

/// The `Browser` data structure is an entry mapped to the
/// a browser program installed on the user's OS. What determines
/// the list of present browser is platform specific.
#[derive(Debug, Clone)]
pub struct Browser {
    // The path to the executable binary or script that is the entry point
    // of the browser program. This path is absolute and free of arguments.
    pub exe_path: String,

    // The arguments that should be passed when executing the browser binary
    pub arguments: Vec<String>,

    // User friendly browser program name, deducted from the executable metadata
    // as defined by the program publisher
    pub name: String,

    // Path to the browser program icon/logo
    pub icon: String,

    pub handle_icon: winapi::HICON,
    pub exe_exists: bool,
    pub icon_exists: bool,
    pub version: VersionInfo,
}

impl Default for Browser {
    fn default() -> Browser {
        Browser {
            exe_path: String::default(),
            arguments: Vec::default(),
            name: String::default(),
            version: VersionInfo::default(),
            icon: String::default(),
            exe_exists: false,
            icon_exists: false,
            handle_icon: std::ptr::null_mut(),
        }
    }
}


impl TryInto<ListItem<Browser>> for &Browser {
    type Error = crate::error::BSError;
    fn try_into(self) -> Result<ListItem<Browser>> {
        let image =
            BrowserSelectorUI::<Browser>::load_image(self.exe_path.as_str())
                .unwrap_or_default();

        let uuid = {
            let mut hasher = DefaultHasher::new();
            self.exe_path.hash(&mut hasher);
            hasher.finish().to_string()
        };

        Ok(ListItem {
            title: self.version.product_name.clone(),
            subtitle: vec![
                self.version.product_version.clone(),
                self.version.binary_type.to_string(),
                self.version.company_name.clone(),
                self.version.file_description.clone(),
            ]
            .into_iter()
            .filter(|itm| itm.len() > 0)
            .collect::<Vec<String>>()
            .join(" | "),
            image,
            uuid,
            state: std::rc::Rc::new(self.clone()),
        })
    }
}

#[derive(Debug, Default)]
struct WinExePath {
    pub path_to_exe: String,
    pub arguments: Vec<String>,
}

/// Windows paths can sometimes be formatted with arguments
/// in the form of "C:\Path\To\Exe" --arg1 --arg2
/// this method converts it into path string and arguments array
impl From<&str> for WinExePath {
    fn from(string_path: &str) -> Self {
        // TODO: Support dobule quote escaped arguments "someArg"
        // TODO: Use WinAPI to do this instead using this:
        // https://docs.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shevaluatesystemcommandtemplate
        if let [_, exe_path, args_part, ..] =
            *string_path.split('"').collect::<Vec<&str>>().as_slice()
        {
            let arguments = match args_part.len() {
                len if len > 0 => args_part.trim().split(' ').collect::<Vec<&str>>(),
                _ => Vec::default(),
            };
            let arguments_len = arguments.len();

            return WinExePath {
                path_to_exe: String::from(exe_path),
                arguments: arguments.into_iter().fold(
                    Vec::with_capacity(arguments_len),
                    |mut arg_list, arg| {
                        arg_list.push(String::from(arg));
                        arg_list
                    },
                ),
            };
        }

        WinExePath {
            path_to_exe: String::from(string_path),
            arguments: Vec::default(),
        }
    }
}

pub fn read_system_browsers_sync() -> Result<Vec<Browser>> {
    // windows registry
    let path32 = "SOFTWARE\\Clients\\StartMenuInternet";
    let path64 = "SOFTWARE\\WOW6432Node\\Clients\\StartMenuInternet";
    let mut list = [
        read_browsers_from_reg_path_sync(path32)?,
        read_browsers_from_reg_path_sync(path64)?,
    ]
    .concat();

    // dedup below only compares current with next element
    // lists need to be sorted for dedup_by to work
    list.sort_unstable_by_key(|item| item.exe_path.clone());
    list.dedup_by(|a, b| a.exe_path == b.exe_path);

    for browser in list.iter_mut() {
        let path_and_args = WinExePath::from(browser.exe_path.as_str());
        browser.exe_path = path_and_args.path_to_exe;
        browser.arguments = path_and_args.arguments;

        match read_browser_exe_info(&browser.exe_path) {
            Ok(version) => browser.version = version,
            Err(e) => println!(
                "Error with reading browser info for {}. Reason: {}",
                browser.exe_path, e
            ),
        }

        match crate::os::get_exe_file_icon(&browser.exe_path) {
            Ok(icon) => browser.handle_icon = icon,
            Err(e) => println!(
                "Error loading icon from file {}, Reason: {}",
                browser.exe_path, e
            ),
        }
    }
    Ok(list)
}

fn read_browsers_from_reg_path_sync(win_reg_path: &str) -> Result<Vec<Browser>> {
    let mut browsers: Vec<Browser> = Vec::new();
    let root = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey(win_reg_path)
        .unwrap(); // TODO: grecefully handle error instead of panicing

    for key in root.enum_keys().map(|x| x.unwrap()) {
        match read_browser_info_from_reg_key(&[win_reg_path, "\\", &key].join("")) {
            Ok(browser) => browsers.push(browser),
            Err(e) => println!("Error reading browser info: {:?}", e),
        }
    }
    for (name, value) in root.enum_values().map(|x| x.unwrap()) {
        println!("\t{} = {:?}", name, value);
    }
    Ok(browsers)
}

fn read_browser_info_from_reg_key(reg_path: &str) -> std::io::Result<Browser> {
    let shell_reg_path = "shell\\open\\command";
    let icon_reg_path = "DefaultIcon";

    let browser_root_key =
        winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE).open_subkey(reg_path)?;
    let shell_open_command_key = browser_root_key.open_subkey(shell_reg_path)?;
    let icon_key = browser_root_key.open_subkey(icon_reg_path)?;

    let name: String = browser_root_key.get_value("")?; // empty gives us (Default)
    let exe_path: String = shell_open_command_key.get_value("")?;
    let icon = icon_key.get_value("")?;

    Ok(Browser {
        name,
        exe_path,
        icon,
        ..Browser::default()
    })
}

fn read_browser_exe_info(path: &str) -> Result<VersionInfo> {
    let mut ver_info = read_exe_version_info(path)?;
    ver_info.binary_type = read_exe_arch(path)?;

    Ok(ver_info)
}

/// For the given `path` it returns the architecture of the
/// executable to be either 32 or 64 bits.
fn read_exe_arch(path: &str) -> Result<BinaryType> {
    // WinAPI rust crate is missing the SCS_ constants thus
    // we need to define the values here
    // https://github.com/retep998/winapi-rs/issues/930
    const NONE: u32 = 12345678; // 0 is already assigned to 32 bits by the WinAPI
    const WINAPI_BITS32: u32 = 0;
    const WINAPI_BITS64: u32 = 6;

    let file_path_wide = crate::os::str_to_wide(path);
    let mut bin_type: u32 = NONE;
    let api_call_result =
        unsafe { winapi::GetBinaryTypeW(file_path_wide.as_ptr(), &mut bin_type as *mut u32) };

    if api_call_result < 1 {
        bail!(
            "Cannot read binary type with GetBinaryTypeW for file {}",
            path
        );
    }

    Ok(match bin_type {
        WINAPI_BITS32 => BinaryType::Bits32,
        WINAPI_BITS64 => BinaryType::Bits64,
        _ => BinaryType::None,
    })
}

/// Reads file attributes specific to Windows executables as per the fields
/// in `VersionInfo` struct based on the given `path`.
///
/// The fields read are:
/// - ProductName
/// - CompanyName
/// - ProductVersion
///
/// ### Implementation details
/// The implementation is overly complicated due to the goal of having a correct
/// implementation as per Microsoft Docs, thus what we do is:
///  - ask the OS what size is needed to hold the whole blob containing the file version fields
///  - allocate a buffer with that size and ask the OS to copy the whole blob in our buffer
///  - the blob contains some Windows specific hierarchy structures where the data we're interested in is beneath a certain language code
///  - we ask the OS to tell us what are the metadata languages in the .exe file
///  - we ask the OS what is the OS setting for the user's language and we pick .exe language that matches the UI default or the language neutral entry which Windows defines it as a lang code of 0, or the first element found
///  - we ask the for specific values of the properties `ProductName`, `CompanyName`, `ProductVersion` and if they're not `UTF-16` we convert them based on the indicated `Code Page`.
fn read_exe_version_info(path: &str) -> Result<VersionInfo> {
    const UTF16_WINDOWS_CODE_PAGE: u16 = 1200;
    let file_path_wide = crate::os::str_to_wide(path);
    let file_version_size: u32 =
        unsafe { winapi::GetFileVersionInfoSizeW(file_path_wide.as_ptr(), &mut 0) };
    if file_version_size == 0 {
        bail!(
            "Cannot read file version size with GetFileVersionInfoSizeExW for {}",
            path
        );
    }

    unsafe {
        let mut version_info_blob: Vec<u8> = Vec::with_capacity(file_version_size as usize);
        if winapi::GetFileVersionInfoW(
            file_path_wide.as_ptr(),
            0,
            file_version_size,
            version_info_blob.as_mut_ptr() as *mut winapi::c_void,
        ) == 0
        {
            bail!(
                "Cannot get file version info data with GetFileVersionInfoW for {}",
                path
            );
        }

        #[repr(C)]
        #[derive(Debug)]
        #[allow(non_snake_case)]
        struct LANGANDCODEPAGE {
            wLanguage: u16,
            wCodePage: u16,
        }
        type PCLANGANDCODEPAGE = *const LANGANDCODEPAGE;

        // pointer within `buffer` var above based on the sub block given to VerQueryValueW
        let mut out_pointer = std::ptr::null_mut();

        // the number of bytes VerQueryValueW has written for the the requested sub block from within the `version_info_blob`
        let mut out_size: u32 = 0;

        let translations_sub_block = crate::os::str_to_wide("\\VarFileInfo\\Translation");

        let result = winapi::VerQueryValueW(
            version_info_blob.as_ptr() as *mut winapi::c_void,
            translations_sub_block.as_ptr(),
            &mut out_pointer,
            &mut out_size,
        );

        println!("Address of the verinfo buffer: {:p}", &version_info_blob);
        println!("Address of the VerQueryValue pointer: {:p}", out_pointer);
        let raw_buff =
            std::slice::from_raw_parts::<u8>(out_pointer as *const u8, out_size as usize);
        println!("Raw buffer:\n{:?}", raw_buff);

        if result == 0 || out_size == 0 || out_pointer == std::ptr::null_mut() {
            bail!(
                "Failed to read version info for {}. GetLastError: {:#x}",
                path,
                winapi::GetLastError()
            );
        }

        let translations_len = out_size as usize / std::mem::size_of::<LANGANDCODEPAGE>();

        // TODO: do we need to forget this because it's technically part of the version_info_blob?
        let translations: &[LANGANDCODEPAGE] =
            std::slice::from_raw_parts(out_pointer as PCLANGANDCODEPAGE, translations_len);

        let user_lang_id = winapi::GetUserDefaultUILanguage();
        let default_lang_id = 0; // 0 means language neutral

        // look at the translations list and find the one matching
        // the OS language (user_lang_id) or find the language neutral one (default_lang_id)
        // or just return the first element (&translations[0])
        let translation: &LANGANDCODEPAGE = translations
            .iter()
            .find(|item| item.wLanguage == user_lang_id)
            .unwrap_or_else(|| {
                translations
                    .iter()
                    .find(|item| item.wLanguage == default_lang_id)
                    .unwrap_or_else(|| &translations[0])
            });

        let base_block = format!(
            "\\StringFileInfo\\{:04x}{:04x}",
            translation.wLanguage, translation.wCodePage
        );
        let product_name_block = base_block.clone() + "\\ProductName";
        let company_name_block = base_block.clone() + "\\CompanyName";
        let product_version_block = base_block.clone() + "\\ProductVersion";

        let mut results = Vec::<String>::with_capacity(3);

        for &block in [
            &product_name_block,
            &company_name_block,
            &product_version_block,
        ]
        .iter()
        {
            // pointer within `buffer` var above based on the sub block given to VerQueryValueW
            let mut out_pointer = std::ptr::null_mut();

            // the number of bytes VerQueryValueW has written for the the requested sub block from within the `version_info_blob`
            let mut out_size: u32 = 0;
            let result = winapi::VerQueryValueW(
                version_info_blob.as_ptr() as *mut winapi::c_void,
                crate::os::str_to_wide(block).as_ptr(),
                &mut out_pointer,
                &mut out_size,
            );

            if result == 0 || out_size == 0 || out_pointer == std::ptr::null_mut() {
                results.push(String::from(""));
                continue;
            }

            let raw_wide_string: Vec<u16>;
            if translation.wCodePage != UTF16_WINDOWS_CODE_PAGE {
                // TODO: do we need to std::mem::forget this because it's technically part of the version_info_blob?
                let raw_string =
                    std::slice::from_raw_parts(out_pointer as *const i8, out_size as usize)
                        .to_vec();
                raw_wide_string = crate::os::ansi_str_to_wide(&raw_string, translation.wCodePage)?;
            } else {
                raw_wide_string =
                    std::slice::from_raw_parts(out_pointer as *const u16, out_size as usize)
                        .to_vec();
            }

            let result_str = crate::os::wide_to_str(&raw_wide_string);
            results.push(result_str);
        }

        if let [product_name, company_name, product_version] = results.as_slice() {
            return Ok(VersionInfo {
                product_name: product_name.into(),
                product_version: product_version.into(),
                company_name: company_name.into(),
                ..Default::default()
            });
        } else {
            bail!("Not all required props were found.");
        }
    }
}
