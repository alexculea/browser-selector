// https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Workspace/Articles/InformationAboutFiles.html#//apple_ref/doc/uid/20001004-CJBIDCEF
use crate::{error::BSResult, os::shared::VersionInfo, ui::ListItem};
use std::fs::*;

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
            let info_plist_path = file.unwrap().path().join("Contents").join("Info.plist");
            if !info_plist_path.exists() {
                return;
            }

            if let Some(app_info_dict) = plist::Value::from_file(info_plist_path)
                .unwrap()
                .as_dictionary()
            {
                if let Some(supported_url_types) = app_info_dict.get("CFBundleURLTypes") {
                    if let Some(urls) = supported_url_types.as_array() {
                        urls.iter().for_each(| url | {
                            if let Some(url_string) = url.as_string() {
                                if urls_required.contains(&url_string) {
                                    browsers.push(browser_from_plist(app_info_dict).unwrap())
                                }
                            }
                        })
                    }
                }
            }

            // if let app_info.get(key)
        })
    });
    // for dir in directories {
    //     let files = read_dir(dir).unwrap();
    //     files.map
    // }

    Ok(Default::default())
}

fn browser_from_plist(dict: &plist::Dictionary) -> BSResult<Browser> {
    let exe_path = if let Some(path) = dict.get("CFBundleShortVersionString") {
        path.as_string()
    } else { None };

    // Browser {
    //     exe_path: 
    // }
}
