use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("文件不存在或已被移动，可以从列表中移除该链接")]
    FileMissing,
    #[error("该文件夹已经绑定过了，可以在侧边栏中管理已有绑定")]
    BindingExists,
    #[error("文件夹不存在或无法访问，请确认路径后重新扫描，或解除绑定")]
    BindingUnavailable,
    #[error("无法打开文件，请确认文件仍在原位置，或从列表中移除该链接")]
    OpenFailed(String),
    #[error("找不到对应的记录，请刷新后重试")]
    NotFound,
    #[error("至少需要一个文件类型，请填写后再保存")]
    EmptyExtensions,
    #[error("保存数据失败，请检查磁盘空间和文件权限后重试")]
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
    use std::io;

    #[test]
    fn file_missing_serializes_code_and_message() {
        let value = serde_json::to_value(AppError::FileMissing).unwrap();
        assert_eq!(
            value,
            json!({
                "code": "FileMissing",
                "message": "文件不存在或已被移动，可以从列表中移除该链接"
            })
        );
    }

    #[test]
    fn user_messages_have_no_english_and_include_next_step() {
        let messages = [
            AppError::FileMissing.to_string(),
            AppError::BindingExists.to_string(),
            AppError::BindingUnavailable.to_string(),
            AppError::OpenFailed("Access is denied".into()).to_string(),
            AppError::NotFound.to_string(),
            AppError::EmptyExtensions.to_string(),
            AppError::Io(io::Error::other("disk full")).to_string(),
        ];
        for message in messages {
            assert!(
                !message.chars().any(|c| c.is_ascii_alphabetic()),
                "english leaked: {message}"
            );
            assert!(
                message.contains('，') || message.contains('。'),
                "missing next step: {message}"
            );
        }
    }
}
