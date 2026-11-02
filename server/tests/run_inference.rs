//! Verifies the `InferenceService` run channel end to end through the local router.
#[path = "support/fake_provider.rs"]
mod fake_provider;
#[path = "support/fixtures.rs"]
mod fixtures;

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use cursor_server::{
    api::cursor,
    cursor::{
        prompting::{PromptAssets, PromptCompiler},
        protocol::{
            connect,
            inference::{
                inference_core_message, inference_stream_response, run_inference_client_message,
                run_inference_server_message, BidiPollRequest, BidiPollResponse,
                InferenceCoreMessage, InferenceMessageRole, InferenceRequestedModel,
                InferenceStreamRequest, RunInferenceCancelInvocation, RunInferenceClientMessage,
                RunInferenceFinishRun, RunInferenceInvokeModel, RunInferenceRunRequest,
                RunInferenceServerMessage,
            },
            proto::aiserver::v1 as ai,
        },
        transport::TransportRegistry,
    },
    network::NetworkClients,
    provider::{FinishReason, ModelEvent},
    store::Store,
};
use prost::Message;
use tower::ServiceExt;

const REQUEST_ID: &str = "run-inference-test";
const MODEL_ID: &str = "plugin:test/openai/gpt-4o-mini";

fn transport(store: Store, provider: &fake_provider::FakeProvider) -> TransportRegistry {
    let assets = PromptAssets::load(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("prompt/cursor")
            .as_path(),
    )
    .unwrap();
    TransportRegistry::new(
        store,
        Arc::new(provider.clone()),
        PromptCompiler::new(assets),
    )
}

fn run_request() -> RunInferenceClientMessage {
    RunInferenceClientMessage {
        message: Some(run_inference_client_message::Message::RunRequest(
            RunInferenceRunRequest {
                conversation_id: "conversation-1".into(),
                requested_model: Some(InferenceRequestedModel {
                    model_id: MODEL_ID.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )),
    }
}

fn invoke_request(invocation_id: &str) -> RunInferenceClientMessage {
    RunInferenceClientMessage {
        message: Some(run_inference_client_message::Message::InvokeModel(
            RunInferenceInvokeModel {
                invocation_id: invocation_id.into(),
                request: Some(InferenceStreamRequest {
                    messages: vec![InferenceCoreMessage {
                        role: InferenceMessageRole::User as i32,
                        content: Some(inference_core_message::Content::Text("hi".into())),
                        ..Default::default()
                    }],
                    model_id: Some(MODEL_ID.into()),
                    ..Default::default()
                }),
            },
        )),
    }
}

fn finish_run() -> RunInferenceClientMessage {
    RunInferenceClientMessage {
        message: Some(run_inference_client_message::Message::FinishRun(
            RunInferenceFinishRun {
                aborted: false,
                turn_completed: Some(true),
            },
        )),
    }
}

#[allow(dead_code)]
fn cancel_invocation(invocation_id: &str) -> RunInferenceClientMessage {
    RunInferenceClientMessage {
        message: Some(run_inference_client_message::Message::CancelInvocation(
            RunInferenceCancelInvocation {
                invocation_id: invocation_id.into(),
            },
        )),
    }
}

async fn append(router: &axum::Router, seqno: i64, message: &RunInferenceClientMessage) -> StatusCode {
    let request = ai::BidiAppendRequest {
        data: hex::encode(message.encode_to_vec()),
        request_id: Some(ai::BidiRequestId {
            request_id: REQUEST_ID.into(),
        }),
        append_seqno: seqno,
        ..Default::default()
    };
    let response = router
        .clone()
        .oneshot(
            Request::post("/aiserver.v1.BidiService/BidiAppend")
                .header(header::CONTENT_TYPE, "application/connect+proto")
                .body(Body::from(connect::encode_message(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    response.status()
}

async fn poll(router: &axum::Router, start: bool) -> Vec<BidiPollResponse> {
    let request = BidiPollRequest {
        request_id: Some(ai::BidiRequestId {
            request_id: REQUEST_ID.into(),
        }),
        start_request: Some(start),
    };
    let response = router
        .clone()
        .oneshot(
            Request::post("/aiserver.v1.InferenceService/RunInferencePoll")
                .header(header::CONTENT_TYPE, "application/connect+proto")
                .body(Body::from(connect::encode_message(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    connect::decode_frames(&body)
        .unwrap()
        .into_iter()
        .filter(|(flags, _)| *flags == 0)
        .map(|(_, payload)| BidiPollResponse::decode(payload).unwrap())
        .collect()
}

fn server_messages(frames: &[BidiPollResponse]) -> Vec<RunInferenceServerMessage> {
    frames
        .iter()
        .filter(|frame| !frame.eof.unwrap_or(false))
        .map(|frame| RunInferenceServerMessage::decode(hex::decode(&frame.data).unwrap().as_slice()).unwrap())
        .collect()
}

#[tokio::test]
async fn run_inference_streams_provider_events_as_server_messages() {
    let (_directory, store) = fixtures::temp_store().await;
    let provider = fake_provider::FakeProvider::default();
    provider.push(vec![
        ModelEvent::TextDelta("hello".into()),
        ModelEvent::TextEnd,
        ModelEvent::Done(FinishReason::Stop),
    ]);
    let registry = transport(store.clone(), &provider);
    let router = cursor::router(registry, NetworkClients::new(store)).unwrap();

    assert_eq!(append(&router, 1, &run_request()).await, StatusCode::OK);
    assert_eq!(
        append(&router, 2, &invoke_request("invocation-1")).await,
        StatusCode::OK
    );

    let mut frames = Vec::new();
    for attempt in 0..10 {
        let batch = poll(&router, attempt == 0).await;
        let ended = batch.iter().any(|frame| frame.eof.unwrap_or(false));
        frames.extend(batch);
        if ended || server_messages(&frames).iter().any(|message| {
            matches!(
                message.message,
                Some(run_inference_server_message::Message::InvocationEnd(_))
            )
        }) {
            break;
        }
    }

    let messages = server_messages(&frames);
    assert!(
        matches!(
            messages.first().map(|message| &message.message),
            Some(Some(run_inference_server_message::Message::RunReady(_)))
        ),
        "the run must be acknowledged before any invocation frame"
    );
    let text: String = messages
        .iter()
        .filter_map(|message| match &message.message {
            Some(run_inference_server_message::Message::InvocationResponse(response)) => {
                match response.response.as_ref()?.response.as_ref()? {
                    inference_stream_response::Response::TextPart(part) => Some(part.text.clone()),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(text, "hello");
    assert!(
        messages.iter().any(|message| matches!(
            &message.message,
            Some(run_inference_server_message::Message::InvocationEnd(end)) if end.error.is_none()
        )),
        "the invocation must end without an error after the provider stream finishes"
    );

    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].history.len(), 1);
    assert_eq!(requests[0].model.model_id, MODEL_ID);
}

#[tokio::test]
async fn finishing_the_run_marks_the_poll_stream_as_eof() {
    let (_directory, store) = fixtures::temp_store().await;
    let provider = fake_provider::FakeProvider::default();
    let registry = transport(store.clone(), &provider);
    let router = cursor::router(registry, NetworkClients::new(store)).unwrap();

    assert_eq!(append(&router, 1, &run_request()).await, StatusCode::OK);
    assert_eq!(append(&router, 2, &finish_run()).await, StatusCode::OK);

    let frames = poll(&router, true).await;
    assert!(
        frames.iter().any(|frame| frame.eof.unwrap_or(false)),
        "finishing the run must terminate the poll stream"
    );
}

#[tokio::test]
async fn legacy_bidi_append_still_reports_an_unknown_payload() {
    let (_directory, store) = fixtures::temp_store().await;
    let provider = fake_provider::FakeProvider::default();
    let registry = transport(store.clone(), &provider);
    let router = cursor::router(registry, NetworkClients::new(store)).unwrap();

    let request = ai::BidiAppendRequest {
        data: hex::encode([0xffu8, 0xff, 0xff]),
        request_id: Some(ai::BidiRequestId {
            request_id: REQUEST_ID.into(),
        }),
        append_seqno: 1,
        ..Default::default()
    };
    let response = router
        .clone()
        .oneshot(
            Request::post("/aiserver.v1.BidiService/BidiAppend")
                .header(header::CONTENT_TYPE, "application/connect+proto")
                .body(Body::from(connect::encode_message(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "an undecodable append must not be reported as success"
    );
}
