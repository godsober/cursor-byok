//! Implements advertisement configuration endpoints.
//! Advertisement service contract and desktop HTTP handler.
//!
//! 广告位为内置静态内容：仅保留「作者寄语」与「中转站推荐」两个卡片，
//! 不再从远端广告服务拉取，也不再下发任何第三方广告。

use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::Result;

use super::ControlService;

const AUTHOR_NOTE_AD_ID: &str = "author-note";
const RELAY_PROMO_AD_ID: &str = "relay-promo";

const AUTHOR_NOTE_IMAGE: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath fill='%2339D9E7' d='M4 3h16a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H9l-5 4V5a2 2 0 0 1 2-2z'/%3E%3C/svg%3E";
const RELAY_PROMO_IMAGE: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cg fill='none' stroke='%2339D9E7' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m19.25 7.65-2.55-3.2a1.33 1.33 0 0 0-1.03-.5H8.58c-.34 0-.67.13-.91.37L4.15 7.65c-.93.88-.6 1.52.65 2.3l9.55 5.97'/%3E%3Cpath d='m4.75 16.35 2.55 3.2c.25.31.63.5 1.03.5h7.09c.34 0 .67-.13.91-.37l3.52-3.33c.93-.88.6-1.52-.65-2.3L9.65 8.08'/%3E%3C/g%3E%3C/svg%3E";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdRuntime {
    pub slots: Vec<AdSlot>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdSlot {
    pub id: String,
    pub enabled: bool,
    pub placement: AdPlacement,
    pub target: AdTarget,
    pub content: AdContent,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdPlacement {
    Menu,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdTarget {
    pub title: String,
    pub description: String,
    pub image_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdContent {
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub details: Vec<AdDetail>,
    pub button: Option<AdButton>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdDetail {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdButton {
    pub label: String,
    pub action: AdAction,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdAction {
    #[serde(rename = "type")]
    pub action_type: AdActionType,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdDismissalInput {
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdActionType {
    OpenBrowser,
}

/// 返回内置静态广告位：作者寄语 + 中转站推荐（点击跳转 token-ai.site）。
pub(super) fn static_runtime() -> AdRuntime {
    AdRuntime {
        slots: vec![
            AdSlot {
                id: AUTHOR_NOTE_AD_ID.to_owned(),
                enabled: true,
                placement: AdPlacement::Menu,
                target: AdTarget {
                    title: "作者寄语".into(),
                    description: "推荐使用 soberApi 源头稳定 价格抄底".into(),
                    image_url: AUTHOR_NOTE_IMAGE.into(),
                },
                content: AdContent {
                    title: "作者寄语".into(),
                    description: "推荐使用 soberApi，源头稳定，价格抄底。".into(),
                    image_url: AUTHOR_NOTE_IMAGE.into(),
                    details: Vec::new(),
                    button: None,
                },
            },
            AdSlot {
                id: RELAY_PROMO_AD_ID.to_owned(),
                enabled: true,
                placement: AdPlacement::Menu,
                target: AdTarget {
                    title: "中转站推荐".into(),
                    description: "Token AI 中转站 · token-ai.site".into(),
                    image_url: RELAY_PROMO_IMAGE.into(),
                },
                content: AdContent {
                    title: "Token AI 中转站".into(),
                    description: "域名 + API Key 直连，兼容 Anthropic / OpenAI 协议，Claude、GPT 等全系模型通用。".into(),
                    image_url: RELAY_PROMO_IMAGE.into(),
                    details: vec![
                        AdDetail {
                            label: "地址".into(),
                            value: "https://token-ai.site".into(),
                        },
                        AdDetail {
                            label: "接入方式".into(),
                            value: "域名 + API Key，兼容 Anthropic / OpenAI 协议".into(),
                        },
                    ],
                    button: Some(AdButton {
                        label: "立即访问".into(),
                        action: AdAction {
                            action_type: AdActionType::OpenBrowser,
                            url: "https://token-ai.site".into(),
                        },
                    }),
                },
            },
        ],
    }
}

pub async fn get(State(service): State<ControlService>) -> Result<Json<AdRuntime>> {
    Ok(Json(service.ads().await?))
}

pub async fn dismiss(
    State(service): State<ControlService>,
    Path(ad_id): Path<String>,
    Json(input): Json<AdDismissalInput>,
) -> Result<StatusCode> {
    service.dismiss_ad(&ad_id, &input).await?;
    Ok(StatusCode::NO_CONTENT)
}
