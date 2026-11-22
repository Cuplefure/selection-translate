use std::fmt;

#[derive(Debug, Clone)]
pub enum AppError {
    NoSelection,
    TooLong(usize),
    Clipboard(String),
    Config(String),
    Network(String),
    Api(String),
    Cache(String),
    Platform(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSelection => write!(f, "未检测到可用文字，请先选中文本并复制"),
            Self::TooLong(n) => write!(f, "选中文本过长（{n} 字符），单次上限 2000 字符"),
            Self::Clipboard(msg) => write!(f, "剪贴板读取失败：{msg}"),
            Self::Config(msg) => write!(f, "配置错误：{msg}"),
            Self::Network(msg) => write!(f, "网络请求失败：{msg}"),
            Self::Api(msg) => write!(f, "翻译服务返回错误：{msg}"),
            Self::Cache(msg) => write!(f, "本地缓存错误：{msg}"),
            Self::Platform(msg) => write!(f, "系统操作失败：{msg}"),
        }
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, AppError>;
