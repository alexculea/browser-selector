#[allow(dead_code)]
pub type BSResult<T> = std::result::Result<T, BSError>;

#[derive(Debug, Default)]
pub struct BSError {
    msg: String,
}

impl BSError {
    pub fn new(msg: &str) -> BSError {
        BSError { msg: String::from(msg) }
    }
}

impl std::fmt::Display for BSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.msg.as_str())
    }
}

#[cfg(target_os = "windows")]
impl From<winrt::Error> for BSError {
    fn from(err: winrt::Error) -> Self {
        BSError::new(format!("[WinRT error] code: {} {}", err.code().0, err.message(),).as_str())
    }
}

impl From<simple_error::SimpleError> for BSError {
    fn from(err: simple_error::SimpleError) -> Self {
        BSError::new(err.as_str())
    }
}

impl From<serde_yaml::Error> for BSError {
    fn from(err: serde_yaml::Error) -> Self {
        BSError::new(&err.to_string())
    }
}

impl From<&str> for BSError {
    fn from(str: &str) -> Self {
        BSError::new(str)
    }
}

impl From<std::io::Error> for BSError {
    fn from(error: std::io::Error) -> Self {
        let os_code = error.raw_os_error().unwrap_or(0);
        let error_message = format!("IO Error: Code ({}) - {}", os_code, error.to_string());
        BSError::new(error_message.as_str())
    }
}

impl From<rusqlite::Error> for BSError {
    fn from(error: rusqlite::Error) -> Self {
        let error_message = format!("SQLite Error:- {}", error.to_string());
        BSError::new(error_message.as_str())
    }
}



// impl From<std::io::Result<T>> for BSResult<T> {
//     fn from(result: std::io::Result<T>) -> Self {
//         match result {
//             Ok(t) => Ok(t),
//             Err(e) => e.into()
//         }
//     }
// }