//! Converts `InferenceService` wire messages to local model requests and events.
use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD, Engine};
use prost_types::value::Kind;
use prost_types::{Struct, Value};
use serde_json::json;

use crate::{
    cursor::protocol::inference::{
        inference_core_message::Content, inference_content_part::Part,
        inference_stream_response, InferenceAgentTool, InferenceCoreMessage, InferenceMessageRole,
        InferenceStreamRequest, InferenceTextStreamPart, InferenceThinkingStreamPart,
        InferenceToolCallStreamPart, InferenceUsageInfo,
    },
    model::{
        normalize_tool_name, ContentPart, ModelRequest, ModelSpec, ProjectedContent,
        ProjectedMessage, PromptSpec, Role, ToolCallContent, ToolDefinition, ToolResultContent,
    },
    provider::ModelEvent,
    Error, Result,
};

/// One `invoke_model` call resolved to the local provider request.
pub struct InvocationRequest {
    pub model_id: String,
    pub model_request: ModelRequest,
    /// Provider-facing tool name to the name Cursor expects back.
    pub tool_names: HashMap<String, String>,
}

/// Provider tool index to the call id and name Cursor expects for that call.
pub type ToolCalls = HashMap<usize, (String, String)>;

pub fn build_invocation(
    request: &InferenceStreamRequest,
    fallback_model: &str,
    run_id: &str,
) -> Result<InvocationRequest> {
    let model_id = request
        .model_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .or_else(|| {
            request
                .requested_model
                .as_ref()
                .map(|model| model.model_id.as_str())
                .filter(|id| !id.is_empty())
        })
        .unwrap_or(fallback_model)
        .to_owned();
    if model_id.is_empty() {
        return Err(Error::Protocol(
            "invoke_model did not name a model".into(),
        ));
    }

    let mut instructions = Vec::new();
    let mut history = Vec::new();
    for (index, message) in request.messages.iter().enumerate() {
        let message_id = format!("{run_id}:{index}");
        convert_message(message, &message_id, &mut instructions, &mut history)?;
    }

    let mut tool_names = HashMap::new();
    let mut tools = Vec::with_capacity(request.tools.len());
    for tool in &request.tools {
        let definition = convert_tool(tool, &mut tool_names);
        tools.push(definition);
    }

    Ok(InvocationRequest {
        model_request: ModelRequest {
            prompt: PromptSpec {
                instructions: instructions.join("\n\n"),
                tools,
            },
            model: ModelSpec::new(model_id.clone()),
            history,
        },
        model_id,
        tool_names,
    })
}

fn convert_message(
    message: &InferenceCoreMessage,
    message_id: &str,
    instructions: &mut Vec<String>,
    history: &mut Vec<ProjectedMessage>,
) -> Result<()> {
    match role_of(message.role) {
        Role::System => {
            let text = message.content_text();
            if !text.is_empty() {
                instructions.push(text);
            }
            Ok(())
        }
        Role::User => {
            let parts = message_parts(message)?;
            if parts.is_empty() {
                return Ok(());
            }
            history.push(ProjectedMessage {
                message_id: message_id.to_owned(),
                role: Role::User,
                content: ProjectedContent::Parts(parts),
            });
            Ok(())
        }
        Role::Assistant => {
            let text = message.content_text();
            let thinking = message
                .reasoning_parts
                .iter()
                .filter(|part| !part.is_redacted)
                .map(|part| part.text.as_str())
                .collect::<String>();
            let calls = message
                .tool_calls
                .iter()
                .enumerate()
                .map(|(index, call)| ToolCallContent {
                    index,
                    call_id: call.tool_call_id.clone(),
                    name: call.tool_name.clone(),
                    arguments: call
                        .args
                        .as_ref()
                        .map(struct_to_json)
                        .or_else(|| {
                            call.raw_tool_call_args
                                .as_deref()
                                .and_then(|raw| serde_json::from_str(raw).ok())
                        })
                        .unwrap_or_else(|| json!({})),
                })
                .collect();
            history.push(ProjectedMessage {
                message_id: message_id.to_owned(),
                role: Role::Assistant,
                content: ProjectedContent::Assistant {
                    text,
                    thinking,
                    replay_state: None,
                    calls,
                },
            });
            Ok(())
        }
        Role::Tool => {
            let Some(Content::ToolContent(results)) = message.content.as_ref() else {
                return Ok(());
            };
            for (index, part) in results.parts.iter().enumerate() {
                history.push(ProjectedMessage {
                    message_id: format!("{message_id}:tool:{index}"),
                    role: Role::Tool,
                    content: ProjectedContent::ToolResult(ToolResultContent {
                        call_id: part.tool_call_id.clone(),
                        name: part.tool_name.clone(),
                        content: value_text(part.result.as_ref()),
                        is_error: part.is_error,
                        image: None,
                        provider_parts: Vec::new(),
                    }),
                });
            }
            Ok(())
        }
    }
}

fn role_of(role: i32) -> Role {
    match InferenceMessageRole::try_from(role) {
        Ok(InferenceMessageRole::System) => Role::System,
        Ok(InferenceMessageRole::Assistant) => Role::Assistant,
        Ok(InferenceMessageRole::Tool) => Role::Tool,
        _ => Role::User,
    }
}

impl InferenceCoreMessage {
    /// Every text part joined in wire order; assistant turns rely on all of them.
    fn content_text(&self) -> String {
        match self.content.as_ref() {
            Some(Content::Text(text)) => text.clone(),
            Some(Content::Parts(parts)) => parts
                .parts
                .iter()
                .filter_map(|part| match part.part.as_ref() {
                    Some(Part::Text(text)) => Some(text.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        }
    }
}

fn message_parts(message: &InferenceCoreMessage) -> Result<Vec<ContentPart>> {
    let mut parts = Vec::new();
    match message.content.as_ref() {
        Some(Content::Text(text)) => parts.push(ContentPart::Text { text: text.clone() }),
        Some(Content::Parts(content)) => {
            for part in &content.parts {
                match part.part.as_ref() {
                    Some(Part::Text(text)) => {
                        parts.push(ContentPart::Text {
                            text: text.text.clone(),
                        });
                    }
                    Some(Part::Image(image)) => {
                        let data = STANDARD.decode(image.data.as_bytes()).map_err(|error| {
                            Error::Protocol(format!("invalid inference image payload: {error}"))
                        })?;
                        parts.push(ContentPart::Image {
                            mime_type: image
                                .mime_type
                                .clone()
                                .unwrap_or_else(|| "image/png".into()),
                            data,
                        });
                    }
                    Some(Part::File(file)) => parts.push(ContentPart::Text {
                        text: format!("[file: {}]", file.filename.as_deref().unwrap_or("attached")),
                    }),
                    None => {}
                }
            }
        }
        _ => {}
    }
    Ok(parts)
}

fn convert_tool(tool: &InferenceAgentTool, tool_names: &mut HashMap<String, String>) -> ToolDefinition {
    let normalized = normalize_tool_name(&tool.name);
    tool_names.insert(normalized.clone(), tool.name.clone());
    ToolDefinition {
        name: normalized,
        description: tool.description.clone(),
        parameters: tool
            .parameters
            .as_ref()
            .map(struct_to_json)
            .unwrap_or_else(|| json!({"type": "object", "properties": {}})),
    }
}

pub fn struct_to_json(value: &Struct) -> serde_json::Value {
    serde_json::Value::Object(
        value
            .fields
            .iter()
            .map(|(name, value)| (name.clone(), value_to_json(value)))
            .collect(),
    )
}

pub fn value_to_json(value: &Value) -> serde_json::Value {
    match value.kind.as_ref() {
        Some(Kind::NullValue(_)) | None => serde_json::Value::Null,
        Some(Kind::NumberValue(number)) => serde_json::Number::from_f64(*number)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Some(Kind::StringValue(text)) => serde_json::Value::String(text.clone()),
        Some(Kind::BoolValue(flag)) => serde_json::Value::Bool(*flag),
        Some(Kind::StructValue(value)) => struct_to_json(value),
        Some(Kind::ListValue(list)) => {
            serde_json::Value::Array(list.values.iter().map(value_to_json).collect())
        }
    }
}

fn value_text(value: Option<&Value>) -> String {
    match value.map(value_to_json) {
        Some(serde_json::Value::String(text)) => text,
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Maps one provider event to the wire response Cursor consumes for an invocation.
pub fn response_for_event(
    event: &ModelEvent,
    tool_names: &HashMap<String, String>,
    calls: &mut ToolCalls,
) -> Option<InferenceStreamResponseBody> {
    let body = match event {
        ModelEvent::TextDelta(text) => InferenceStreamResponseBody::text(text.clone(), false),
        ModelEvent::TextEnd => InferenceStreamResponseBody::text(String::new(), true),
        ModelEvent::ThinkingDelta(text) => {
            InferenceStreamResponseBody::thinking(text.clone(), false)
        }
        ModelEvent::ThinkingEnd => InferenceStreamResponseBody::thinking(String::new(), true),
        ModelEvent::ToolCallStart {
            index,
            call_id,
            name,
        } => {
            let name = tool_names.get(name).cloned().unwrap_or_else(|| name.clone());
            calls.insert(*index, (call_id.clone(), name.clone()));
            InferenceStreamResponseBody::tool_call(InferenceToolCallStreamPart {
                tool_call_id: call_id.clone(),
                tool_name: name,
                args: String::new(),
                is_complete: false,
                tool_index: Some(*index as i32),
            })
        }
        ModelEvent::ToolCallArgumentsDelta { index, delta } => {
            let (call_id, name) = calls.get(index).cloned().unwrap_or_default();
            InferenceStreamResponseBody::tool_call(InferenceToolCallStreamPart {
                tool_call_id: call_id,
                tool_name: name,
                args: delta.clone(),
                is_complete: false,
                tool_index: Some(*index as i32),
            })
        }
        ModelEvent::ToolCallEnd { index } => {
            let (call_id, name) = calls.get(index).cloned().unwrap_or_default();
            InferenceStreamResponseBody::tool_call(InferenceToolCallStreamPart {
                tool_call_id: call_id,
                tool_name: name,
                args: String::new(),
                is_complete: true,
                tool_index: Some(*index as i32),
            })
        }
        ModelEvent::Usage(usage) => InferenceStreamResponseBody::usage(InferenceUsageInfo {
            prompt_tokens: usage.input_tokens.unwrap_or_default() as i32,
            completion_tokens: usage.output_tokens.unwrap_or_default() as i32,
            total_tokens: usage.total_tokens.map(|total| total as i32),
        }),
        ModelEvent::Start { .. } | ModelEvent::TextStart | ModelEvent::ThinkingStart => {
            return None;
        }
        ModelEvent::ProviderReplayState(_) | ModelEvent::Done(_) => return None,
    };
    Some(body)
}

pub enum InferenceStreamResponseBody {
    Text(InferenceTextStreamPart),
    Thinking(InferenceThinkingStreamPart),
    ToolCall(InferenceToolCallStreamPart),
    Usage(InferenceUsageInfo),
}

impl InferenceStreamResponseBody {
    fn text(text: String, is_final: bool) -> Self {
        Self::Text(InferenceTextStreamPart { text, is_final })
    }

    fn thinking(text: String, is_final: bool) -> Self {
        Self::Thinking(InferenceThinkingStreamPart {
            text,
            signature: None,
            is_final,
        })
    }

    fn tool_call(part: InferenceToolCallStreamPart) -> Self {
        Self::ToolCall(part)
    }

    fn usage(usage: InferenceUsageInfo) -> Self {
        Self::Usage(usage)
    }
}

impl From<InferenceStreamResponseBody> for inference_stream_response::Response {
    fn from(body: InferenceStreamResponseBody) -> Self {
        match body {
            InferenceStreamResponseBody::Text(part) => Self::TextPart(part),
            InferenceStreamResponseBody::Thinking(part) => Self::ThinkingPart(part),
            InferenceStreamResponseBody::ToolCall(part) => Self::ToolCallPart(part),
            InferenceStreamResponseBody::Usage(part) => Self::Usage(part),
        }
    }
}
