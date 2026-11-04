use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("文件不存在或已被移动")]
    FileMissing,
    #[error("该文件夹已经绑定过了")]
    BindingExists,
    #[error("文件夹不存在或无法访问")]
    BindingUnavailable,
    #[error("无法打开文件：{0}")]
    OpenFailed(String),
    #[error("找不到对应的记录")]
    NotFound,
    #[error("至少需要一个文件类型")]
    EmptyExtensions,
    #[error("保存数据失败：{0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::FileMissing => "FileMissing",
            Self::BindingExists => "BindingExists",
            Self::BindingUnavailable => "BindingUnavailable",
            Self::OpenFailed(_) => "OpenFailed",
            Self::NotFound => "NotFound",
            Self::EmptyExtensions => "EmptyExtensions",
            Self::Io(_) => "Io",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;
    use serde_json::json;

    #[test]
    fn file_missing_serializes_code_and_message() {
        let value = serde_json::to_value(AppError::FileMissing).unwrap();
        assert_eq!(
            value,
            json!({
                "code": "FileMissing",
                "message": "文件不存在或已被移动"
            })
        );
    }
}
