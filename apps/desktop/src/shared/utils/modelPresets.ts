import type { ModelType } from "../api";
import deepseekIcon from "../assets/provider-icons/deepseek.svg";
import kimiIcon from "../assets/provider-icons/kimi.svg";
import minimaxIcon from "../assets/provider-icons/minimax.svg";
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
/** openai 协议：路径不规则，直接给完整请求 URL，可指定 openaiEndpoint（默认从 URL 提取） */
const openaiFullUrl = (
  url: string,
  openaiEndpoint?: string,
): ModelPresetEndpoint => {
  // 从 URL 中提取端点路径（如 /v1/responses 或 /v1/chat/completions）
  const detectedEndpoint =
    url.match(/\/(v1\/(?:responses|chat\/completions))$/)?.[1] ||
    "/v1/chat/completions";
  return {
    baseUrl: url,
    useFullUrl: true,
    openaiEndpoint: openaiEndpoint || `/${detectedEndpoint}`,
    customHeaders: null,
  };
};

/** openai 协议：标准路径，填 Base URL，自动追加 /v1/chat/completions */
const openaiChat = (baseUrl: string): ModelPresetEndpoint => ({
  baseUrl,
  useFullUrl: false,
  openaiEndpoint: "/v1/chat/completions",
  customHeaders: null,
});

export const modelPresets: ModelPreset[] = [
  {
    key: "openai",
    name: "OpenAI",
    icon: "",
    keyHint: "仅支持 Token AI 中转站，服务器地址已锁定，不可修改。",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiFullUrl("https://token-ai.site/v1/responses"),
    },
    models: [
      entry("gpt-4o", "GPT-4o", 128000, 16384),
      entry("gpt-4o-mini", "GPT-4o Mini", 128000, 16384),
      entry("gpt-4-turbo", "GPT-4 Turbo", 128000, 4096),
      entry("gpt-3.5-turbo", "GPT-3.5 Turbo", 16385, 4096),
    ],
  },
  {
    key: "anthropic",
    name: "Anthropic",
    icon: "",
    keyHint: "仅支持 Token AI 中转站，服务器地址已锁定，不可修改。",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("claude-sonnet-4-5", "Claude Sonnet 4.5", 200000, 8192),
      entry("claude-haiku-3-5", "Claude Haiku 3.5", 200000, 8192),
      entry("claude-opus-4", "Claude Opus 4", 200000, 4096),
    ],
  },
  {
    key: "deepseek",
    name: "DeepSeek",
    icon: deepseekIcon,
    keyHint: "仅支持 Token AI 中转站，服务器地址已锁定，不可修改。",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("deepseek-chat", "DeepSeek Chat", 64000, 8192),
      entry("deepseek-coder", "DeepSeek Coder", 64000, 8192),
    ],
  },
  {
    key: "gemini",
    name: "Gemini",
    icon: "",
    keyHint: "仅支持 Token AI 中转站，服务器地址已锁定，不可修改。",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("gemini-2.5-pro", "Gemini 2.5 Pro", 1000000, 8192),
      entry("gemini-2.5-flash", "Gemini 2.5 Flash", 1000000, 8192),
      entry("gemini-2.0-flash", "Gemini 2.0 Flash", 1000000, 8192),
    ],
  },
  {
    key: "grok",
    name: "Grok",
    icon: "",
    keyHint: "仅支持 Token AI 中转站，服务器地址已锁定，不可修改。",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("grok-3", "Grok 3", 131072, 8192),
      entry("grok-3-mini", "Grok 3 Mini", 131072, 8192),
    ],
  },
  {
    key: "kimi",
    name: "Kimi (Moonshot)",
    icon: kimiIcon,
    keyHint: "仅支持 Token AI 中转站，服务器地址已锁定，不可修改。",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("kimi-latest", "Kimi Latest", 128000, 8192),
      entry("moonshot-v1-128k", "Moonshot V1 128K", 128000, 8192),
      entry("moonshot-v1-32k", "Moonshot V1 32K", 32000, 8192),
    ],
  },
  {
    key: "zhipu",
    name: "智谱 GLM",
    icon: zhipuIcon,
    keyHint: "仅支持 Token AI 中转站，服务器地址已锁定，不可修改。",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [
      entry("glm-4", "GLM-4", 128000, 4096),
      entry("glm-4-plus", "GLM-4-Plus", 128000, 4096),
      entry("glm-4-flash", "GLM-4-Flash", 128000, 4096),
    ],
  },
  {
    key: "minimax",
    name: "MiniMax",
    icon: minimaxIcon,
    keyHint: "仅支持 Token AI 中转站，服务器地址已锁定，不可修改。",
    endpoints: {
      anthropic: anthropic("https://token-ai.site"),
      openai: openaiChat("https://token-ai.site"),
    },
    models: [entry("minimax-text-01", "MiniMax Text 01", 1000000, 8192)],
  },
];

export const trimTrailingSlash = (url: string) => url.replace(/\/+$/, "");

export const presetEndpoint = (
  preset: ModelPreset,
  type: ModelType,
): ModelPresetEndpoint => preset.endpoints[type];
