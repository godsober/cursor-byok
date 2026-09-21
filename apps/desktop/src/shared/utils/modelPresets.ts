import type { ModelType } from "../api";
import deepseekIcon from "../assets/provider-icons/deepseek.svg";
import kimiIcon from "../assets/provider-icons/kimi.svg";
import tokenaiIcon from "../assets/provider-icons/tokenai.svg";
import zhipuIcon from "../assets/provider-icons/zhipu.svg";
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

/** openai 协议：标准路径，填 Base URL，自动追加 /v1/chat/completions */
const openaiChat = (baseUrl: string): ModelPresetEndpoint => ({
  baseUrl,
  useFullUrl: false,
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
  {
    key: "deepseek",
    name: "DeepSeek",
    icon: deepseekIcon,
    keyHint: "通过 Token AI 中转站调用 DeepSeek（使用 Chat Completions API）",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("deepseek-v4-pro", "DeepSeek V4 Pro", 1000000, 65536),
      entry("deepseek-v4-flash", "DeepSeek V4 Flash", 1000000, null),
    ],
  },
  {
    key: "gemini",
    name: "Gemini",
    icon: tokenaiIcon, // 临时使用 tokenai 图标，后续可替换
    keyHint: "通过 Token AI 中转站调用 Gemini（使用 Chat Completions API）",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("gemini-2.5-pro", "Gemini 2.5 Pro", 2000000, 65536),
      entry("gemini-2.5-flash", "Gemini 2.5 Flash", 1000000, 65536),
    ],
  },
  {
    key: "grok",
    name: "Grok",
    icon: tokenaiIcon, // 临时使用 tokenai 图标，后续可替换
    keyHint: "通过 Token AI 中转站调用 Grok（使用 Responses API）",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiFullUrl("https://token-ai.site/v1/responses"),
    },
    models: [
      entry("grok-4", "Grok 4", 2000000, 65536),
      entry("grok-3", "Grok 3", 131072, 65536),
    ],
  },
  {
    key: "kimi",
    name: "Kimi (Moonshot)",
    icon: kimiIcon,
    keyHint: "通过 Token AI 中转站调用 Kimi（使用 Chat Completions API）",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("kimi-latest", "Kimi Latest", 128000, 8192),
      entry("moonshot-v1-128k", "Moonshot V1 128K", 128000, 8192),
    ],
  },
  {
    key: "zhipu",
    name: "智谱 GLM",
    icon: zhipuIcon,
    keyHint: "通过 Token AI 中转站调用智谱 GLM（使用 Chat Completions API）",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("glm-5.3", "GLM 5.3", 1000000, 65536),
      entry("glm-5.2", "GLM 5.2", 200000, 32768),
      entry("glm-4.7", "GLM 4.7", 200000, 32768),
    ],
  },
];

export const trimTrailingSlash = (url: string) => url.replace(/\/+$/, "");

export const presetEndpoint = (
  preset: ModelPreset,
  type: ModelType,
): ModelPresetEndpoint => preset.endpoints[type];
