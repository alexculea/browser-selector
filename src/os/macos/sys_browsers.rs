// https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Workspace/Articles/InformationAboutFiles.html#//apple_ref/doc/uid/20001004-CJBIDCEF
use crate::{
    error::{BSError, BSResult},
    os::shared::{BinaryType, VersionInfo},
    ui::ListItem,
};
use std::{fs::*, path::Path};

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
        }
    }
}

impl TryInto<ListItem<Browser>> for &Browser {
    type Error = crate::error::BSError;
    fn try_into(self) -> BSResult<ListItem<Browser>> {
        // let image =
        //     BrowserSelectorUI::<Browser>::load_image(self.exe_path.as_str())
        //         .unwrap_or_default();

        // let uuid = {
        //     let mut hasher = DefaultHasher::new();
        //     self.exe_path.hash(&mut hasher);
        //     hasher.finish().to_string()
        // };

        // Ok(ListItem {
        //     title: self.version.product_name.clone(),
        //     subtitle: vec![
        //         self.version.product_version.clone(),
        //         self.version.binary_type.to_string(),
        //         self.version.company_name.clone(),
        //         self.version.file_description.clone(),
        //     ]
        //     .into_iter()
        //     .filter(|itm| itm.len() > 0)
        //     .collect::<Vec<String>>()
        //     .join(" | "),
        //     image,
        //     uuid,
        //     state: std::rc::Rc::new(self.clone()),
        // })

        todo!()
    }
}

pub fn read_system_browsers_sync() -> BSResult<Vec<Browser>> {
    // Read /Aplications and /System/Applications
    // For each directory go to <app-folder>/Contents/Info.plist
    // Using a Plist parser, look under CFBundleURLTypes -> CFBundleURLSchemes, see it includes https
    // Reading publisher & Version info as well
    let urls_required = ["https", "http"];
    let directories = ["/Applications", "/System/Applications"];
    let mut browsers: Vec<Browser> = Vec::with_capacity(5);
    directories.iter().for_each(|dir| {
        read_dir(dir).unwrap().for_each(|file| {
            let info_plist_path = file
                .as_ref()
                .unwrap()
                .path()
                .join("Contents")
                .join("Info.plist");
            let app_dir = file.unwrap().path().join("Contents");
            if !info_plist_path.exists() {
                return;
            }

            if let Some(app_info_dict) = plist::Value::from_file(info_plist_path.clone())
                .unwrap()
                .as_dictionary()
            {
                if let Some(supported_url_types) = app_info_dict.get("CFBundleURLTypes") {
                    if let Some(urls) = supported_url_types.as_array() {
                        urls.iter().for_each(|url_entry| {
                            if let Some(url_scheme_entry) = url_entry.as_dictionary() {
                                if let Some(url_schemes) = url_scheme_entry.get("CFBundleURLSchemes") {
                                    if let Some(url_schemes_list) = url_schemes.as_array() {
                                        url_schemes_list.iter().for_each(|url_scheme_entry| {
                                            if let Some(scheme_string) = url_scheme_entry.as_string() {
                                                if urls_required.contains(&scheme_string) {
                                                    let browser_info = browser_from_plist(app_info_dict, &app_dir);
                                                    if browser_info.is_ok() {
                                                        browsers.push(browser_info.unwrap())
                                                    } else {
                                                        println!("Error reading browser info: {}", browser_info.err().unwrap())
                                                    }
                                                }
                                            }
                                        });
                                    }
                                } else {
                                    println!("No CFBundleURLSchemes dictionary found.")
                                }
                            } else {
                                println!("Cannot get CFBundleURLTypes item as dictionary.")
                            }
                        })
                    } else {
                        println!("CFBundleURLTypes dictionary found, but can't retrieve it as an array.");
                    }
                } else {
                    println!("No CFBundleURLTypes dictionary found.")
                }
            }

            println!("Finished reading {}", info_plist_path.clone().to_string_lossy());
            // if let app_info.get(key)
        })
    });
    // for dir in directories {
    //     let files = read_dir(dir).unwrap();
    //     files.map
    // }

    Ok(browsers)
}

fn browser_from_plist(dict: &plist::Dictionary, app_dir: &Path) -> BSResult<Browser> {
    let plist_props = [
        "CFBundleExecutable",
        "CFBundleName",
        "CFBundleShortVersionString",
    ];

    let prop_values = plist_props
        .iter()
        .map(|plist_prop| {
            dict.get(plist_prop)
                .ok_or(BSError::new(&format!(
                    "No {plist_prop} found in Info.plist"
                )))?
                .as_string()
                .ok_or(BSError::new(&format!(
                    "Cannot convert {plist_prop} to a string."
                )))
        })
        .try_fold(Vec::<String>::new(), |mut result, item| {
            if item.is_err() {
                BSResult::Err(item.err().unwrap())
            } else {
                result.push(item.unwrap().to_string());
                BSResult::Ok(result)
            }
        })?;
    let [bin_filename, name, version_code] = prop_values.as_slice() else {
        unreachable!()
    };

    let exe_path = app_dir.join("MacOS").join(bin_filename);
    let exe_path_string = exe_path.to_string_lossy().to_string();
    let exe_exists = exe_path.exists();
    let arguments: Vec<String> = Default::default();
    let icon = String::default();

    let version = VersionInfo {
        company_name: String::default(),
        file_description: String::default(),
        product_version: version_code.to_string(),
        product_name: name.to_string(),
        binary_type: BinaryType::None,
    };

    Ok(Browser {
        exe_path: exe_path_string,
        exe_exists,
        icon_exists: false,
        version,
        name: name.to_string(),
        icon,
        arguments,
    })
}
