// https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Workspace/Articles/InformationAboutFiles.html#//apple_ref/doc/uid/20001004-CJBIDCEF

use crate::{error::BSResult, os::shared::VersionInfo, ui::ListItem};

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
    Ok(Default::default())
}
