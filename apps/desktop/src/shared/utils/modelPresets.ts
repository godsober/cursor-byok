import type { ModelType } from "../api";
import tokenaiIcon from "../assets/provider-icons/tokenai.svg";
import { defaultCustomHeaders } from "./modelDefaults";

export interface ModelPresetEntry {
  model_id: string;
  display_name: string;
  context_window_tokens: number | null;
  max_output_tokens: number | null;
}

/** 一个服务商在某种协议（anthropic / openai）下的接入端点 */
export interface ModelPresetEndpoint {
  baseUrl: string;
  /** true 时 baseUrl 即完整请求 URL；false 时由请求协议追加标准端点路径 */
  useFullUrl: boolean;
  /** openai 协议的请求端点（useFullUrl 为 true 时忽略） */
  openaiEndpoint: string;
  /** 非空时启用自定义 Headers（claude-cli 伪装头） */
  customHeaders: Record<string, string> | null;
}

export interface ModelPreset {
  key: string;
  name: string;
  icon: string;
  keyHint: string;
  /** 五家服务商均同时提供 Anthropic 与 OpenAI 兼容协议 */
  endpoints: { anthropic: ModelPresetEndpoint; openai: ModelPresetEndpoint };
  models: ModelPresetEntry[];
}

const entry = (
  modelId: string,
  displayName: string,
  contextWindowTokens: number | null,
  maxOutputTokens: number | null,
): ModelPresetEntry => ({
  model_id: modelId,
  display_name: displayName,
  context_window_tokens: contextWindowTokens,
  max_output_tokens: maxOutputTokens,
});

const claudeHeaders = { ...defaultCustomHeaders };
/** anthropic 协议：填 Base URL，自动追加 /v1/messages */
const anthropic = (baseUrl: string): ModelPresetEndpoint => ({
  baseUrl,
  useFullUrl: false,
  openaiEndpoint: "",
  customHeaders: claudeHeaders,
});
/** openai 协议：路径不规则，直接给完整请求 URL */
const openaiFullUrl = (url: string): ModelPresetEndpoint => ({
  baseUrl: url,
  useFullUrl: true,
  openaiEndpoint: "/v1/chat/completions",
  customHeaders: null,
});

export const modelPresets: ModelPreset[] = [
  {
    key: "tokenai",
    name: "Token AI 中转站",
    icon: tokenaiIcon,
    keyHint:
      "token-ai.site 控制台获取 API Key（默认 Responses API + 完整请求地址，获取模型需先填入 API Key，返回该密钥可用的模型列表，兼容 Anthropic 与 OpenAI 协议）",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiFullUrl("https://token-ai.site/v1/responses"),
    },
    models: [
      entry("gpt-5.6-sol", "sober-gpt-5.6-sol", 272000, 240000),
      entry("claude-sonnet-4-5", "Claude Sonnet 4.5", 272000, 64000),
      entry("claude-opus-4-6", "Claude Opus 4.6", 272000, 64000),
      entry("gpt-5.5-codex", "GPT-5.5 Codex", 272000, 128000),
      entry("gpt-5.4", "GPT-5.4", 272000, 128000),
    ],
  },
];

export const trimTrailingSlash = (url: string) => url.replace(/\/+$/, "");

export const presetEndpoint = (
  preset: ModelPreset,
  type: ModelType,
): ModelPresetEndpoint => preset.endpoints[type];
