use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct DeeplResponse {
    translations: Vec<DeeplItem>,
}

#[derive(Debug, Deserialize)]
struct DeeplItem {
    text: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
}

pub async fn translate(
    settings: &Settings,
    api_key: Option<&str>,
    text: &str,
    source_lang: &str,
) -> AppResult<String> {
    match settings.provider.as_str() {
        "mock" => Ok(format!("[EN] {}", text)),
        "deepl" => deepl(settings, api_key, text, source_lang).await,
        "openai" => openai(settings, api_key, text).await,
        other => Err(AppError::Config(format!("未知翻译服务：{other}"))),
    }
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .pool_max_idle_per_host(4)
        .build()
        .map_err(|e| AppError::Network(e.to_string()))
}

async fn deepl(
    settings: &Settings,
    api_key: Option<&str>,
    text: &str,
    source_lang: &str,
) -> AppResult<String> {
    let key = api_key.ok_or_else(|| AppError::Api("未配置 DeepL API 密钥".into()))?;
    let form = [
        ("auth_key", key),
        ("text", text),
        ("source_lang", if source_lang == "auto" { "auto" } else { "ZH" }),
        ("target_lang", "EN"),
    ];
    let response = client()?
        .post(&settings.endpoint)
        .form(&form)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| AppError::Api(e.to_string()))?;
    let parsed: DeeplResponse = response
        .json()
        .await
        .map_err(|e| AppError::Api(e.to_string()))?;
    parsed
        .translations
        .first()
        .map(|item| item.text.clone())
        .ok_or_else(|| AppError::Api("响应中没有译文".into()))
}

async fn openai(settings: &Settings, api_key: Option<&str>, text: &str) -> AppResult<String> {
    let key = api_key.ok_or_else(|| AppError::Api("未配置 API 密钥".into()))?;
    let body = serde_json::json!({
        "model": settings.model,
        "temperature": 0,
        "messages": [
            {
                "role": "system",
                "content": "Translate the user text into natural English. Return only the translation."
            },
            { "role": "user", "content": text }
        ]
    });
    let response = client()?
        .post(&settings.endpoint)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| AppError::Api(e.to_string()))?;
    let parsed: OpenAIResponse = response
        .json()
        .await
        .map_err(|e| AppError::Api(e.to_string()))?;
    parsed
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| AppError::Api("响应中没有译文".into()))
}
