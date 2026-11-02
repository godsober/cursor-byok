import { useRef } from "react";
import type { ModelInput, ModelType } from "../../shared/api";
import { defaultCustomHeadersText } from "../../shared/utils/modelDefaults";
import { modelPresets, presetEndpoint } from "../../shared/utils/modelPresets";
import { Button } from "../../shared/ui/Button";
import { Checkbox } from "../../shared/ui/Checkbox";
import {
  FormField,
  SecretTextInput,
  TextInput,
} from "../../shared/ui/FormControls";
import { JsonEditor } from "../../shared/ui/JsonEditor";
import { Combobox, Select, type ComboboxHandle } from "../../shared/ui/Select";
import { Switch } from "../../shared/ui/Switch";
import styles from "./CursorSettings.module.scss";

export type CursorModelDraft = {
  providerId: string;
  model: ModelInput;
  openAIExtraParamsText: string;
  customHeadersText: string;
  anthropicExtraParamsText: string;
};

export const emptyCursorModelDraft = (): CursorModelDraft => ({
  providerId: "builtin/openai",
  model: {
    sort_order: 0,
    display_name: "",
    group_name: null,
    type: "openai",
    base_url: "https://token-ai.site/v1/responses",
    use_full_url: true,
    api_key: "",
    tooltip_data: "",
    model_id: "",
    reasoning_effort: null,
    openai_endpoint: "/v1/responses",
    anthropic_endpoint: "/v1/messages",
    openai_extra_params_enabled: false,
    openai_extra_params: {},
    custom_headers_enabled: false,
    custom_headers: {},
    anthropic_extra_params_enabled: false,
    anthropic_extra_params: {},
    context_window_tokens: null,
    max_completion_tokens: null,
    anthropic_max_tokens: null,
    anthropic_thinking_effort: "xhigh",
    thinking_budget_tokens: null,
  },
  openAIExtraParamsText: "{}",
  customHeadersText: defaultCustomHeadersText,
  anthropicExtraParamsText: "{}",
});

export function CursorModelEditor({
  draft,
  modelOptions,
  discovering,
  onChange,
  onDiscover,
}: {
  draft: CursorModelDraft;
  modelOptions: string[];
  discovering: boolean;
  onChange: (draft: CursorModelDraft) => void;
  onDiscover: () => Promise<boolean>;
}) {
  const modelCombobox = useRef<ComboboxHandle>(null);
  const setModel = (patch: Partial<ModelInput>) =>
    onChange({ ...draft, model: { ...draft.model, ...patch } });
  const numberValue = (value: string) =>
    value === "" ? null : Math.trunc(Number(value));
  const canDiscover = Boolean(
    draft.model.base_url.trim() && draft.model.api_key.trim(),
  );
  // 根据当前选择的服务商（providerId）获取对应的模型列表，而不是根据 URL 匹配
  const currentPreset = modelPresets.find(
    (preset) => `builtin/${preset.key}` === draft.providerId,
  );
  const presetModelOptions = currentPreset
    ? currentPreset.models.map((item) => item.model_id)
    : [];
  const combinedOptions = [
    ...new Set([...modelOptions, ...presetModelOptions]),
  ];
  const discoverModels = async () => {
    if (await onDiscover()) modelCombobox.current?.openAll();
  };
  const requestUrlPlaceholder = draft.model.use_full_url
    ? draft.model.type === "anthropic"
      ? "https://api.anthropic.com/v1/messages"
      : draft.model.openai_endpoint === "/v1/chat/completions"
        ? "https://api.openai.com/v1/chat/completions"
        : "https://api.openai.com/v1/responses"
    : draft.model.type === "anthropic"
      ? "https://api.anthropic.com"
      : "https://api.openai.com";

  const providerOptions = modelPresets.map((preset) => ({
    value: `builtin/${preset.key}`,
    label: preset.name,
  }));
  const setProvider = (providerId: string) => {
    const preset = modelPresets.find((p) => `builtin/${p.key}` === providerId);
    if (!preset) return;
    // 如果选择的是 Anthropic 服务商，自动切换到 Anthropic 协议
    const modelType: ModelType =
      preset.key === "anthropic" ? "anthropic" : draft.model.type;
    const endpoint = presetEndpoint(preset, modelType);
    const first = preset.models[0];
    onChange({
      ...draft,
      providerId,
      model: {
        ...draft.model,
        type: modelType,
        base_url: endpoint.baseUrl,
        use_full_url: endpoint.useFullUrl,
        openai_endpoint: modelType === "openai" ? endpoint.openaiEndpoint : "",
        anthropic_endpoint: modelType === "anthropic" ? "/v1/messages" : "",
        custom_headers_enabled: endpoint.customHeaders !== null,
        custom_headers: endpoint.customHeaders
          ? { ...endpoint.customHeaders }
          : {},
        api_key: "", // 切换服务商时清空 API Key
        model_id: first?.model_id ?? draft.model.model_id,
        display_name: first?.display_name ?? draft.model.display_name,
        tooltip_data: preset.name,
        context_window_tokens:
          first?.context_window_tokens ?? draft.model.context_window_tokens,
        ...(modelType === "openai"
          ? {
              max_completion_tokens:
                first?.max_output_tokens ?? draft.model.max_completion_tokens,
            }
          : {
              anthropic_max_tokens:
                first?.max_output_tokens ?? draft.model.anthropic_max_tokens,
            }),
      },
      customHeadersText: endpoint.customHeaders
        ? JSON.stringify(endpoint.customHeaders, null, 2)
        : draft.customHeadersText,
    });
  };

  return (
    <div className={styles.editor}>
      <div className={styles.grid}>
        <FormField label={t("模型类型")}>
          <Select
            ariaLabel={t("模型类型")}
            value={draft.providerId}
            options={providerOptions}
            onChange={setProvider}
          />
        </FormField>
        {draft.model.type === "openai" && (
          <FormField
            label={t("请求协议")}
            hint={t(
              "只决定请求与响应的格式，请求地址会随之更新为 Token AI 中转站对应端点。",
            )}
          >
            {" "}
            <Select
              ariaLabel={t("请求协议")}
              value={draft.model.openai_endpoint}
              options={[
                { value: "/v1/responses", label: "Responses API" },
                {
                  value: "/v1/chat/completions",
                  label: "Chat Completions API",
                },
              ]}
              onChange={(openai_endpoint) => {
                // 根据当前服务商的 baseUrl 更新端点
                const currentPreset = modelPresets.find(
                  (p) => `builtin/${p.key}` === draft.providerId,
                );
                const baseUrl = currentPreset
                  ? presetEndpoint(currentPreset, "openai").baseUrl
                  : "https://token-ai.site";
                const newBaseUrl = baseUrl.replace(
                  /\/(?:v1\/)?(?:responses|chat\/completions)$/,
                  openai_endpoint,
                );
                setModel({
                  openai_endpoint,
                  base_url: newBaseUrl,
                });
              }}
            />
          </FormField>
        )}
        {draft.model.type === "anthropic" && (
          <FormField
            label={t("请求协议")}
            hint={t(
              "只决定请求与响应的格式，请求地址会随之更新为 Token AI 中转站对应端点。",
            )}
          >
            {" "}
            <Select
              ariaLabel={t("请求协议")}
              value={draft.model.anthropic_endpoint}
              options={[{ value: "/v1/messages", label: "Messages API" }]}
              onChange={(anthropic_endpoint) => {
                const currentPreset = modelPresets.find(
                  (p) => `builtin/${p.key}` === draft.providerId,
                );
                const baseUrl = currentPreset
                  ? presetEndpoint(currentPreset, "anthropic").baseUrl
                  : "https://token-ai.site";
                const newBaseUrl = baseUrl.replace(
                  /\/(?:v1\/)?(?:messages)$/,
                  anthropic_endpoint,
                );
                setModel({
                  anthropic_endpoint,
                  base_url: newBaseUrl,
                });
              }}
            />
          </FormField>
        )}

        <div className={styles.urlField}>
          <FormField
            label={
              draft.model.use_full_url ? t("完整请求 URL") : t("服务器地址")
            }
            hint={t("仅支持 Token AI 中转站，服务器地址已锁定，不可修改。")}
          >
            {" "}
            <TextInput
              readOnly
              placeholder={requestUrlPlaceholder}
              value={draft.model.base_url}
              onChange={(event) => setModel({ base_url: event.target.value })}
            />
          </FormField>
          <Checkbox
            checked={draft.model.use_full_url}
            disabled
            label={t("使用完整请求地址")}
            onChange={(use_full_url) => setModel({ use_full_url })}
          />
        </div>
        <FormField label="API Key" hint={t("访问模型服务所需的密钥。")}>
          {" "}
          <SecretTextInput
            placeholder="sk-xxxxxx"
            autoComplete="off"
            value={draft.model.api_key}
            onChange={(event) => setModel({ api_key: event.target.value })}
          />
        </FormField>

        <FormField
          label={t("模型名称")}
          hint={t("可以直接输入模型标识，也可以读取接口返回的模型列表。")}
        >
          <Combobox
            ref={modelCombobox}
            value={draft.model.model_id}
            options={combinedOptions}
            placeholder="gpt-5"
            append={
              <Button
                className={styles.discoverButton}
                disabled={discovering || !canDiscover}
                onClick={() => void discoverModels()}
              >
                {discovering ? t("获取中…") : t("获取模型")}
              </Button>
            }
            onChange={(model_id) => {
              // 如果选择的是 Claude 模型，自动切换到 Anthropic 协议
              const isClaudeModel = model_id.toLowerCase().includes("claude");
              if (isClaudeModel && draft.model.type !== "anthropic") {
                const currentPreset = modelPresets.find(
                  (p) => `builtin/${p.key}` === draft.providerId,
                );
                const endpoint = currentPreset
                  ? presetEndpoint(currentPreset, "anthropic")
                  : {
                      baseUrl: "https://token-ai.site",
                      useFullUrl: false,
                      openaiEndpoint: "",
                      customHeaders: null,
                    };
                onChange({
                  ...draft,
                  model: {
                    ...draft.model,
                    type: "anthropic",
                    model_id,
                    display_name: draft.model.display_name || model_id,
                    base_url: endpoint.baseUrl,
                    use_full_url: endpoint.useFullUrl,
                    anthropic_endpoint: "/v1/messages",
                  },
                });
              } else {
                setModel({
                  model_id,
                  display_name: draft.model.display_name || model_id,
                });
              }
            }}
          />
        </FormField>
        <FormField
          label={t("显示名称")}
          hint={t("仅用于界面展示，不会改变发送给模型服务的模型名称。")}
        >
          {" "}
          <TextInput
            placeholder={t("例如：主力模型")}
            value={draft.model.display_name}
            onChange={(event) => setModel({ display_name: event.target.value })}
          />
        </FormField>
        <FormField
          className={styles.fullWidth}
          label={t("备注")}
          hint={t("显示在 Cursor 模型说明中。")}
        >
          {" "}
          <TextInput
            placeholder={t("请输入模型备注")}
            value={draft.model.tooltip_data}
            onChange={(event) => setModel({ tooltip_data: event.target.value })}
          />
        </FormField>

        <FormField label={t("上下文窗口 Token")} hint={t("留空时使用默认值。")}>
          {" "}
          <TextInput
            type="number"
            min={1}
            step={1}
            placeholder={t("留空使用默认值")}
            value={draft.model.context_window_tokens ?? ""}
            onChange={(event) =>
              setModel({
                context_window_tokens: numberValue(event.target.value),
              })
            }
          />
        </FormField>
        {draft.model.type === "openai" ? (
          <>
            <FormField
              label={t("最大输出 Token")}
              hint={t("留空时使用默认值。")}
            >
              {" "}
              <TextInput
                type="number"
                min={1}
                step={1}
                placeholder={t("留空使用默认值")}
                value={draft.model.max_completion_tokens ?? ""}
                onChange={(event) =>
                  setModel({
                    max_completion_tokens: numberValue(event.target.value),
                  })
                }
              />
            </FormField>
            <FormField label={t("推理强度")}>
              {" "}
              <Select
                ariaLabel={t("推理强度")}
                value={draft.model.reasoning_effort ?? ""}
                options={effortOptions(true)}
                onChange={(value) =>
                  setModel({ reasoning_effort: value || null })
                }
              />
            </FormField>
          </>
        ) : (
          <>
            <FormField
              label={t("最大输出 Token")}
              hint={t("留空时使用默认值。")}
            >
              {" "}
              <TextInput
                type="number"
                min={1}
                step={1}
                placeholder={t("留空使用默认值")}
                value={draft.model.anthropic_max_tokens ?? ""}
                onChange={(event) =>
                  setModel({
                    anthropic_max_tokens: numberValue(event.target.value),
                  })
                }
              />
            </FormField>
            <FormField label={t("思考强度")}>
              {" "}
              <Select
                ariaLabel={t("思考强度")}
                value={draft.model.anthropic_thinking_effort ?? "xhigh"}
                options={effortOptions(false)}
                onChange={(anthropic_thinking_effort) =>
                  setModel({ anthropic_thinking_effort })
                }
              />
            </FormField>
            <FormField
              label={t("思考预算 Token")}
              hint={t("留空时使用 adaptive thinking。")}
            >
              {" "}
              <TextInput
                type="number"
                min={1}
                step={1}
                placeholder={t("留空使用 adaptive thinking")}
                value={draft.model.thinking_budget_tokens ?? ""}
                onChange={(event) =>
                  setModel({
                    thinking_budget_tokens: numberValue(event.target.value),
                  })
                }
              />
            </FormField>
          </>
        )}

        <ToggleJsonField
          label={t("自定义 Headers")}
          enabled={draft.model.custom_headers_enabled}
          text={draft.customHeadersText}
          onEnabledChange={(custom_headers_enabled) =>
            setModel({ custom_headers_enabled })
          }
          onTextChange={(customHeadersText) =>
            onChange({ ...draft, customHeadersText })
          }
        />
        {draft.model.type === "openai" ? (
          <ToggleJsonField
            label={t("OpenAI 额外参数")}
            enabled={draft.model.openai_extra_params_enabled}
            text={draft.openAIExtraParamsText}
            onEnabledChange={(openai_extra_params_enabled) =>
              setModel({ openai_extra_params_enabled })
            }
            onTextChange={(openAIExtraParamsText) =>
              onChange({ ...draft, openAIExtraParamsText })
            }
          />
        ) : (
          <ToggleJsonField
            label={t("Anthropic 额外参数")}
            enabled={draft.model.anthropic_extra_params_enabled}
            text={draft.anthropicExtraParamsText}
            onEnabledChange={(anthropic_extra_params_enabled) =>
              setModel({ anthropic_extra_params_enabled })
            }
            onTextChange={(anthropicExtraParamsText) =>
              onChange({ ...draft, anthropicExtraParamsText })
            }
          />
        )}
      </div>
    </div>
  );
}

function ToggleJsonField({
  label,
  enabled,
  text,
  onEnabledChange,
  onTextChange,
}: {
  label: string;
  enabled: boolean;
  text: string;
  onEnabledChange: (enabled: boolean) => void;
  onTextChange: (text: string) => void;
}) {
  return (
    <div className={`${styles.fullWidth} ${styles.jsonOption}`}>
      <label>
        <span>{label}</span>
        <Switch label={label} checked={enabled} onChange={onEnabledChange} />
      </label>
      {enabled && (
        <JsonEditor ariaLabel={label} value={text} onChange={onTextChange} />
      )}
    </div>
  );
}

function effortOptions(optional: boolean) {
  return [
    ...(optional ? [{ value: "", label: t("不设置") }] : []),
    { value: "low", label: "Low" },
    { value: "medium", label: "Medium" },
    { value: "high", label: "High" },
    { value: "xhigh", label: "Extra High" },
    { value: "max", label: "Max" },
  ];
}
