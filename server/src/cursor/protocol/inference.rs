//! Hand-written `aiserver.v1` types for the `InferenceService` run channel.
//!
//! `protocols/cursor/aiserver_v1.proto` contains the full captured schema but is
//! not compiled as a whole, so the subset used by the local `RunInference`
//! implementation is declared here with the captured field numbers.
use prost_types::{Struct, Value};

use super::proto::aiserver::v1::BidiRequestId;

/// `aiserver.v1.RunInferenceClientMessage`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceClientMessage {
    #[prost(oneof = "run_inference_client_message::Message", tags = "1, 2, 3, 4")]
    pub message: Option<run_inference_client_message::Message>,
}

pub mod run_inference_client_message {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Message {
        #[prost(message, tag = "1")]
        RunRequest(super::RunInferenceRunRequest),
        #[prost(message, tag = "2")]
        InvokeModel(super::RunInferenceInvokeModel),
        #[prost(message, tag = "3")]
        CancelInvocation(super::RunInferenceCancelInvocation),
        #[prost(message, tag = "4")]
        FinishRun(super::RunInferenceFinishRun),
    }
}

/// `aiserver.v1.RunInferenceRunRequest`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceRunRequest {
    #[prost(string, tag = "1")]
    pub conversation_id: String,
    #[prost(string, optional, tag = "2")]
    pub conversation_group_id: Option<String>,
    #[prost(message, optional, tag = "3")]
    pub requested_model: Option<InferenceRequestedModel>,
    #[prost(message, repeated, tag = "4")]
    pub routing_conversation: Vec<RunInferenceRoutingMessage>,
    #[prost(string, optional, tag = "5")]
    pub agent_mode: Option<String>,
    #[prost(string, optional, tag = "6")]
    pub subagent_type_name: Option<String>,
}

/// `aiserver.v1.RunInferenceRoutingMessage`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceRoutingMessage {
    #[prost(enumeration = "InferenceMessageRole", tag = "1")]
    pub role: i32,
    #[prost(string, tag = "2")]
    pub text: String,
}

/// `aiserver.v1.RunInferenceInvokeModel`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceInvokeModel {
    #[prost(string, tag = "1")]
    pub invocation_id: String,
    #[prost(message, optional, tag = "2")]
    pub request: Option<InferenceStreamRequest>,
}

/// `aiserver.v1.RunInferenceCancelInvocation`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceCancelInvocation {
    #[prost(string, tag = "1")]
    pub invocation_id: String,
}

/// `aiserver.v1.RunInferenceFinishRun`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceFinishRun {
    #[prost(bool, tag = "1")]
    pub aborted: bool,
    #[prost(bool, optional, tag = "2")]
    pub turn_completed: Option<bool>,
}

/// `aiserver.v1.RunInferenceServerMessage`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceServerMessage {
    #[prost(oneof = "run_inference_server_message::Message", tags = "1, 2, 3, 4")]
    pub message: Option<run_inference_server_message::Message>,
}

pub mod run_inference_server_message {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Message {
        #[prost(message, tag = "1")]
        Heartbeat(super::RunInferenceHeartbeat),
        #[prost(message, tag = "2")]
        RunReady(super::RunInferenceRunReady),
        #[prost(message, tag = "3")]
        InvocationResponse(super::RunInferenceInvocationResponse),
        #[prost(message, tag = "4")]
        InvocationEnd(super::RunInferenceInvocationEnd),
    }
}

/// `aiserver.v1.RunInferenceHeartbeat`
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct RunInferenceHeartbeat {
    #[prost(bool, tag = "1")]
    pub accepts_speculative_invocations: bool,
    #[prost(bool, tag = "2")]
    pub accepts_compressed_speculative_invocations: bool,
}

/// `aiserver.v1.RunInferencePromptModelMetadata`
///
/// The harness renders prompts locally, so the backend has to name the prompt
/// family it would have used. Only the family selectors are modelled; every
/// capability flag stays false, which selects the generic prompt variant.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferencePromptModelMetadata {
    #[prost(string, tag = "1")]
    pub vendor: String,
    #[prost(string, tag = "2")]
    pub prompt_version: String,
}

/// `aiserver.v1.RunInferenceRunReady`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceRunReady {
    #[prost(message, optional, tag = "1")]
    pub resolved_model: Option<InferenceRequestedModel>,
    #[prost(bool, tag = "2")]
    pub supports_self_summary: bool,
    #[prost(string, optional, tag = "3")]
    pub routed_model_display_name: Option<String>,
    #[prost(message, optional, tag = "4")]
    pub prompt_model_metadata: Option<RunInferencePromptModelMetadata>,
    #[prost(bool, tag = "7")]
    pub accepts_client_heartbeat: bool,
    #[prost(bool, tag = "8")]
    pub accepts_agent_events: bool,
    #[prost(bool, tag = "15")]
    pub accepts_tool_call_events: bool,
    #[prost(bool, tag = "16")]
    pub accepts_client_first_call_timings: bool,
}

/// `aiserver.v1.RunInferenceInvocationResponse`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceInvocationResponse {
    #[prost(string, tag = "1")]
    pub invocation_id: String,
    #[prost(message, optional, tag = "2")]
    pub response: Option<InferenceStreamResponse>,
}

/// `aiserver.v1.RunInferenceInvocationEnd`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceInvocationEnd {
    #[prost(string, tag = "1")]
    pub invocation_id: String,
    #[prost(message, optional, tag = "2")]
    pub error: Option<RunInferenceInvocationError>,
}

/// `aiserver.v1.RunInferenceInvocationError`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RunInferenceInvocationError {
    #[prost(int32, tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub message: String,
}

/// `aiserver.v1.BidiPollRequest`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BidiPollRequest {
    #[prost(message, optional, tag = "1")]
    pub request_id: Option<BidiRequestId>,
    #[prost(bool, optional, tag = "2")]
    pub start_request: Option<bool>,
}

/// `aiserver.v1.BidiPollResponse`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BidiPollResponse {
    #[prost(int64, tag = "1")]
    pub seqno: i64,
    #[prost(string, tag = "2")]
    pub data: String,
    #[prost(bool, optional, tag = "3")]
    pub eof: Option<bool>,
}

/// `aiserver.v1.InferenceStreamRequest`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceStreamRequest {
    #[prost(message, repeated, tag = "1")]
    pub messages: Vec<InferenceCoreMessage>,
    #[prost(message, repeated, tag = "2")]
    pub tools: Vec<InferenceAgentTool>,
    #[prost(message, repeated, tag = "3")]
    pub provider_defined_tools: Vec<InferenceNamedProviderDefinedTool>,
    #[prost(message, optional, tag = "4")]
    pub model_config: Option<InferenceModelConfig>,
    #[prost(string, optional, tag = "5")]
    pub model_id: Option<String>,
    #[prost(string, optional, tag = "6")]
    pub invocation_id: Option<String>,
    #[prost(message, optional, tag = "7")]
    pub requested_model: Option<InferenceRequestedModel>,
    #[prost(string, optional, tag = "8")]
    pub conversation_id: Option<String>,
}

/// `aiserver.v1.InferenceCoreMessage`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceCoreMessage {
    #[prost(enumeration = "InferenceMessageRole", tag = "1")]
    pub role: i32,
    #[prost(message, repeated, tag = "4")]
    pub tool_calls: Vec<InferenceToolCall>,
    #[prost(message, repeated, tag = "7")]
    pub reasoning_parts: Vec<InferenceReasoningPart>,
    #[prost(string, optional, tag = "8")]
    pub model_provider_message_id: Option<String>,
    #[prost(oneof = "inference_core_message::Content", tags = "2, 3, 6")]
    pub content: Option<inference_core_message::Content>,
}

pub mod inference_core_message {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Content {
        #[prost(string, tag = "2")]
        Text(String),
        #[prost(message, tag = "3")]
        Parts(super::InferenceContentParts),
        #[prost(message, tag = "6")]
        ToolContent(super::InferenceToolResultContent),
    }
}

/// `aiserver.v1.InferenceContentParts`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceContentParts {
    #[prost(message, repeated, tag = "1")]
    pub parts: Vec<InferenceContentPart>,
}

/// `aiserver.v1.InferenceContentPart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceContentPart {
    #[prost(oneof = "inference_content_part::Part", tags = "1, 2, 3")]
    pub part: Option<inference_content_part::Part>,
}

pub mod inference_content_part {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Part {
        #[prost(message, tag = "1")]
        Text(super::InferenceTextPart),
        #[prost(message, tag = "2")]
        Image(super::InferenceImagePart),
        #[prost(message, tag = "3")]
        File(super::InferenceFilePart),
    }
}

/// `aiserver.v1.InferenceTextPart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceTextPart {
    #[prost(string, tag = "1")]
    pub text: String,
}

/// `aiserver.v1.InferenceImagePart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceImagePart {
    #[prost(string, tag = "1")]
    pub data: String,
    #[prost(string, optional, tag = "2")]
    pub mime_type: Option<String>,
}

/// `aiserver.v1.InferenceFilePart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceFilePart {
    #[prost(string, tag = "1")]
    pub data: String,
    #[prost(string, tag = "2")]
    pub media_type: String,
    #[prost(string, optional, tag = "3")]
    pub filename: Option<String>,
}

/// `aiserver.v1.InferenceToolCall`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceToolCall {
    #[prost(string, tag = "1")]
    pub tool_call_id: String,
    #[prost(string, tag = "2")]
    pub tool_name: String,
    #[prost(message, optional, tag = "3")]
    pub args: Option<Struct>,
    #[prost(string, optional, tag = "4")]
    pub raw_tool_call_args: Option<String>,
}

/// `aiserver.v1.InferenceToolResultContent`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceToolResultContent {
    #[prost(message, repeated, tag = "1")]
    pub parts: Vec<InferenceToolResultPart>,
}

/// `aiserver.v1.InferenceToolResultPart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceToolResultPart {
    #[prost(string, tag = "1")]
    pub tool_call_id: String,
    #[prost(string, tag = "2")]
    pub tool_name: String,
    #[prost(message, optional, tag = "3")]
    pub result: Option<Value>,
    #[prost(bool, tag = "4")]
    pub is_error: bool,
    #[prost(message, repeated, tag = "5")]
    pub experimental_content: Vec<InferenceContentPart>,
}

/// `aiserver.v1.InferenceReasoningPart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceReasoningPart {
    #[prost(bool, tag = "1")]
    pub is_redacted: bool,
    #[prost(string, tag = "2")]
    pub text: String,
    #[prost(string, optional, tag = "3")]
    pub signature: Option<String>,
    #[prost(string, optional, tag = "5")]
    pub model_name: Option<String>,
}

/// `aiserver.v1.InferenceAgentTool`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceAgentTool {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub description: String,
    #[prost(message, optional, tag = "3")]
    pub parameters: Option<Struct>,
}

/// `aiserver.v1.InferenceNamedProviderDefinedTool`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceNamedProviderDefinedTool {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub id: String,
    #[prost(string, tag = "3")]
    pub r#type: String,
    #[prost(message, optional, tag = "4")]
    pub options: Option<Struct>,
}

/// `aiserver.v1.InferenceModelConfig`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceModelConfig {
    #[prost(int32, optional, tag = "1")]
    pub max_tokens: Option<i32>,
    #[prost(float, optional, tag = "2")]
    pub temperature: Option<f32>,
    #[prost(float, optional, tag = "3")]
    pub top_p: Option<f32>,
    #[prost(string, repeated, tag = "4")]
    pub stop_sequences: Vec<String>,
}

/// `aiserver.v1.InferenceRequestedModel`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceRequestedModel {
    #[prost(string, tag = "1")]
    pub model_id: String,
    #[prost(bool, tag = "2")]
    pub max_mode: bool,
    #[prost(message, repeated, tag = "3")]
    pub parameters: Vec<InferenceModelParameterValue>,
    #[prost(bool, tag = "4")]
    pub built_in_model: bool,
}

/// `aiserver.v1.InferenceModelParameterValue`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceModelParameterValue {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

/// `aiserver.v1.InferenceStreamResponse`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceStreamResponse {
    #[prost(oneof = "inference_stream_response::Response", tags = "1, 2, 3, 8, 9")]
    pub response: Option<inference_stream_response::Response>,
}

pub mod inference_stream_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Response {
        #[prost(message, tag = "1")]
        TextPart(super::InferenceTextStreamPart),
        #[prost(message, tag = "2")]
        ToolCallPart(super::InferenceToolCallStreamPart),
        #[prost(message, tag = "3")]
        Usage(super::InferenceUsageInfo),
        #[prost(message, tag = "8")]
        Error(super::InferenceStreamError),
        #[prost(message, tag = "9")]
        ThinkingPart(super::InferenceThinkingStreamPart),
    }
}

/// `aiserver.v1.InferenceTextStreamPart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceTextStreamPart {
    #[prost(string, tag = "1")]
    pub text: String,
    #[prost(bool, tag = "2")]
    pub is_final: bool,
}

/// `aiserver.v1.InferenceThinkingStreamPart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceThinkingStreamPart {
    #[prost(string, tag = "1")]
    pub text: String,
    #[prost(string, optional, tag = "2")]
    pub signature: Option<String>,
    #[prost(bool, tag = "3")]
    pub is_final: bool,
}

/// `aiserver.v1.InferenceToolCallStreamPart`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceToolCallStreamPart {
    #[prost(string, tag = "1")]
    pub tool_call_id: String,
    #[prost(string, tag = "2")]
    pub tool_name: String,
    #[prost(string, tag = "3")]
    pub args: String,
    #[prost(bool, tag = "4")]
    pub is_complete: bool,
    #[prost(int32, optional, tag = "5")]
    pub tool_index: Option<i32>,
}

/// `aiserver.v1.InferenceUsageInfo`
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct InferenceUsageInfo {
    #[prost(int32, tag = "1")]
    pub prompt_tokens: i32,
    #[prost(int32, tag = "2")]
    pub completion_tokens: i32,
    #[prost(int32, optional, tag = "3")]
    pub total_tokens: Option<i32>,
}

/// `aiserver.v1.InferenceStreamError`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InferenceStreamError {
    #[prost(string, tag = "1")]
    pub message: String,
    #[prost(string, tag = "2")]
    pub code: String,
    #[prost(bool, tag = "3")]
    pub is_input_token_limit_error: bool,
    #[prost(bool, tag = "4")]
    pub is_output_token_limit_error: bool,
    #[prost(enumeration = "InferenceStreamErrorType", tag = "5")]
    pub error_type: i32,
}

/// `aiserver.v1.InferenceStreamErrorType`
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration,
)]
#[repr(i32)]
pub enum InferenceStreamErrorType {
    Unspecified = 0,
    Unknown = 1,
    InputTokenLimit = 2,
    OutputTokenLimit = 3,
    RateLimit = 4,
    Authentication = 5,
    Permission = 6,
    Overloaded = 7,
    ContentFilter = 8,
}

/// `aiserver.v1.InferenceMessageRole`
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration,
)]
#[repr(i32)]
pub enum InferenceMessageRole {
    Unspecified = 0,
    User = 1,
    Assistant = 2,
    Tool = 3,
    System = 4,
}
