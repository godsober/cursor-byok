//! Drives local provider calls for Cursor `InferenceService` runs.
//!
//! Cursor's current agent harness sends a `RunInferenceRunRequest`, then one
//! `RunInferenceInvokeModel` per model turn, and reads `RunInferenceServerMessage`
//! frames back. The harness owns prompting and tool execution; this module only
//! forwards the prepared `InferenceStreamRequest` to the configured provider and
//! streams the normalized events back.
use std::{collections::HashMap, sync::Arc, time::Duration};

use bytes::Bytes;
use futures_util::StreamExt;
use prost::Message;
use tokio::sync::{broadcast, Mutex, Notify};
use tokio_util::sync::CancellationToken;

use crate::{
    cursor::protocol::inference::{
        run_inference_client_message, run_inference_server_message, InferenceStreamRequest,
        InferenceStreamResponse, RunInferenceClientMessage, RunInferenceHeartbeat,
        RunInferenceInvocationEnd, RunInferenceInvocationError,
        RunInferenceInvocationResponse, RunInferenceRunReady, RunInferenceRunRequest,
        RunInferenceServerMessage,
    },
    model::ModelInvocation,
    provider::{ModelEvent, Provider},
    store::Store,
    Error, Result,
};

use super::convert;

/// How long a `RunInferencePoll` request waits for new frames before replying empty.
const POLL_WAIT: Duration = Duration::from_secs(20);
/// How long a finished session stays addressable for late poll/SSE reconnects.
const SESSION_LINGER: Duration = Duration::from_secs(120);

/// One server frame in the per-request sequence.
pub struct Frame {
    pub seqno: i64,
    pub payload: Bytes,
    pub eof: bool,
}

#[derive(Clone)]
pub struct InferenceRegistry {
    inner: Arc<RegistryInner>,
}

struct RegistryInner {
    store: Store,
    provider: Arc<dyn Provider>,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
}

impl InferenceRegistry {
    pub fn new(store: Store, provider: Arc<dyn Provider>) -> Self {
        let registry = Self {
            inner: Arc::new(RegistryInner {
                store,
                provider,
                sessions: Mutex::new(HashMap::new()),
            }),
        };
        registry.spawn_reaper();
        registry
    }

    pub async fn session(&self, request_id: &str) -> Arc<Session> {
        let mut sessions = self.inner.sessions.lock().await;
        if let Some(session) = sessions.get(request_id) {
            return session.clone();
        }
        let session = Arc::new(Session::new(
            request_id.to_owned(),
            self.inner.store.clone(),
            self.inner.provider.clone(),
        ));
        sessions.insert(request_id.to_owned(), session.clone());
        session
    }

    /// Whether this run id already belongs to the `RunInference` channel.
    pub async fn owns(&self, request_id: &str) -> bool {
        self.inner.sessions.lock().await.contains_key(request_id)
    }

    /// Retires a request that only ever carried legacy agent messages.
    pub async fn forget(&self, request_id: &str) {
        let mut sessions = self.inner.sessions.lock().await;
        let empty = match sessions.get(request_id) {
            Some(session) => session.is_empty().await,
            None => false,
        };
        if empty {
            sessions.remove(request_id);
        }
    }

    fn spawn_reaper(&self) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let mut sessions = inner.sessions.lock().await;
                let stale = sessions
                    .iter()
                    .filter(|(_, session)| session.is_stale())
                    .map(|(request_id, _)| request_id.clone())
                    .collect::<Vec<_>>();
                for request_id in stale {
                    sessions.remove(&request_id);
                }
            }
        });
    }

    pub async fn shutdown(&self) {
        let sessions = std::mem::take(&mut *self.inner.sessions.lock().await);
        for session in sessions.into_values() {
            session.cancel().await;
        }
    }
}

struct SessionState {
    frames: Vec<Arc<Frame>>,
    poll_cursor: usize,
    finished: bool,
    finished_at: Option<std::time::Instant>,
    conversation_id: String,
    model_id: String,
    provider_calls: u64,
    active: Option<(String, CancellationToken)>,
}

pub struct Session {
    request_id: String,
    store: Store,
    provider: Arc<dyn Provider>,
    state: Mutex<SessionState>,
    changed: Notify,
    feed: broadcast::Sender<Arc<Frame>>,
}

impl Session {
    fn new(request_id: String, store: Store, provider: Arc<dyn Provider>) -> Self {
        let (feed, _) = broadcast::channel(256);
        Self {
            request_id,
            store,
            provider,
            state: Mutex::new(SessionState {
                frames: Vec::new(),
                poll_cursor: 0,
                finished: false,
                finished_at: None,
                conversation_id: String::new(),
                model_id: String::new(),
                provider_calls: 0,
                active: None,
            }),
            changed: Notify::new(),
            feed,
        }
    }

    async fn is_empty(&self) -> bool {
        self.state.lock().await.frames.is_empty()
    }

    fn is_stale(&self) -> bool {
        let state = self.state.try_lock();
        let Ok(state) = state else {
            return false;
        };
        state
            .finished_at
            .is_some_and(|at| at.elapsed() > SESSION_LINGER)
    }

    async fn cancel(&self) {
        let mut state = self.state.lock().await;
        if let Some((_, cancellation)) = state.active.take() {
            cancellation.cancel();
        }
    }

    pub async fn append(self: Arc<Self>, message: RunInferenceClientMessage) -> Result<()> {
        match message.message {
            Some(run_inference_client_message::Message::RunRequest(request)) => {
                self.start_run(request).await
            }
            Some(run_inference_client_message::Message::InvokeModel(invoke)) => {
                let Some(request) = invoke.request else {
                    return Err(Error::Protocol(
                        "RunInferenceInvokeModel did not include a stream request".into(),
                    ));
                };
                // The provider stream runs in the background: `BidiAppend` must answer
                // immediately while frames arrive through the run stream.
                tokio::spawn(self.invoke(invoke.invocation_id, request));
                Ok(())
            }
            Some(run_inference_client_message::Message::CancelInvocation(cancel)) => {
                self.cancel_invocation(&cancel.invocation_id).await;
                Ok(())
            }
            Some(run_inference_client_message::Message::FinishRun(finish)) => {
                tracing::info!(
                    request_id = self.request_id,
                    aborted = finish.aborted,
                    "RunInference client finished the run"
                );
                self.finish().await;
                Ok(())
            }
            None => {
                // Unmodelled client messages (heartbeats, agent events, logs) are
                // valid traffic for a run and carry nothing the local provider needs.
                tracing::debug!(
                    request_id = self.request_id,
                    "ignoring RunInference client message without a modelled body"
                );
                Ok(())
            }
        }
    }

    async fn start_run(&self, request: RunInferenceRunRequest) -> Result<()> {
        // Echo the requested model back: the harness compares it with its own
        // selection, so it must keep every field the client sent.
        let requested = request.requested_model.clone().unwrap_or_default();
        let model_id = requested.model_id.clone();
        tracing::info!(
            request_id = self.request_id,
            conversation_id = request.conversation_id,
            model_id,
            agent_mode = request.agent_mode.as_deref().unwrap_or_default(),
            routing_messages = request.routing_conversation.len(),
            "RunInference run started"
        );
        let configured_model = if model_id.is_empty() {
            None
        } else {
            self.store.model(&model_id).await.ok().flatten()
        };
        let display_name = configured_model
            .as_ref()
            .map(|model| model.display_name.clone());
        // The harness builds prompts itself, so it needs the prompt family the
        // configured upstream model belongs to.
        let prompt_model_metadata = convert::prompt_model_metadata(
            configured_model
                .as_ref()
                .map(|model| model.model_id.as_str())
                .unwrap_or(&model_id),
        );
        {
            let mut state = self.state.lock().await;
            state.conversation_id = request.conversation_id;
            state.model_id = model_id;
        }
        self.push(run_inference_server_message::Message::RunReady(
            RunInferenceRunReady {
                resolved_model: Some(requested),
                supports_self_summary: false,
                routed_model_display_name: display_name,
                prompt_model_metadata: Some(prompt_model_metadata),
                accepts_client_heartbeat: false,
                accepts_agent_events: false,
                accepts_tool_call_events: false,
                accepts_client_first_call_timings: false,
            },
        ))
        .await;
        Ok(())
    }

    async fn invoke(self: Arc<Self>, invocation_id: String, request: InferenceStreamRequest) {
        let (conversation_id, provider_calls, fallback_model) = {
            let state = self.state.lock().await;
            (
                state.conversation_id.clone(),
                state.provider_calls,
                state.model_id.clone(),
            )
        };
        let run_id = format!("inference:{}", self.request_id);
        let built = match convert::build_invocation(&request, &fallback_model, &run_id) {
            Ok(built) => built,
            Err(error) => {
                self.push_invocation_error(&invocation_id, &error.to_string())
                    .await;
                return;
            }
        };
        let invocation = ModelInvocation {
            call_id: format!("{run_id}:{provider_calls}"),
            run_id,
            conversation_id,
            provider_call_index: provider_calls,
            request: built.model_request,
        };
        let cancellation = CancellationToken::new();
        {
            let mut state = self.state.lock().await;
            if let Some((_, previous)) = state
                .active
                .replace((invocation_id.clone(), cancellation.clone()))
            {
                previous.cancel();
            }
            state.provider_calls += 1;
            state.model_id = built.model_id.clone();
        }
        tracing::info!(
            request_id = self.request_id,
            model_id = built.model_id,
            invocation_id,
            messages = invocation.request.history.len(),
            tools = invocation.request.prompt.tools.len(),
            "RunInference invoking local provider"
        );
        let mut stream = self.provider.stream(invocation, cancellation.clone());
        let mut calls = convert::ToolCalls::new();
        let mut failure = None;
        while let Some(event) = stream.next().await {
            if cancellation.is_cancelled() {
                break;
            }
            match event {
                Ok(event) => {
                    if let Some(response) =
                        convert::response_for_event(&event, &built.tool_names, &mut calls)
                    {
                        self.push_invocation_response(&invocation_id, response).await;
                    }
                    if matches!(event, ModelEvent::Done(_)) {
                        break;
                    }
                }
                Err(error) => {
                    failure = Some(error.to_string());
                    break;
                }
            }
        }
        self.clear_active(&invocation_id).await;
        match failure {
            Some(message) => self.push_invocation_error(&invocation_id, &message).await,
            None => self.push_invocation_end(&invocation_id).await,
        }
    }

    async fn clear_active(&self, invocation_id: &str) {
        let mut state = self.state.lock().await;
        if state
            .active
            .as_ref()
            .is_some_and(|(active, _)| active == invocation_id)
        {
            state.active = None;
        }
    }

    async fn cancel_invocation(&self, invocation_id: &str) {
        let mut state = self.state.lock().await;
        if let Some((active, cancellation)) = state.active.as_ref() {
            if active == invocation_id {
                cancellation.cancel();
                state.active = None;
            }
        }
    }

    async fn finish(&self) {
        self.cancel().await;
        {
            let mut state = self.state.lock().await;
            if state.finished {
                return;
            }
            state.finished = true;
            state.finished_at = Some(std::time::Instant::now());
        }
        self.push_frame(Bytes::new(), true).await;
    }

    async fn push_invocation_response(
        &self,
        invocation_id: &str,
        body: convert::InferenceStreamResponseBody,
    ) {
        self.push(run_inference_server_message::Message::InvocationResponse(
            RunInferenceInvocationResponse {
                invocation_id: invocation_id.to_owned(),
                response: Some(InferenceStreamResponse {
                    response: Some(body.into()),
                }),
            },
        ))
        .await;
    }

    async fn push_invocation_end(&self, invocation_id: &str) {
        self.push(run_inference_server_message::Message::InvocationEnd(
            RunInferenceInvocationEnd {
                invocation_id: invocation_id.to_owned(),
                error: None,
            },
        ))
        .await;
    }

    async fn push_invocation_error(&self, invocation_id: &str, message: &str) {
        tracing::warn!(
            request_id = self.request_id,
            invocation_id,
            %message,
            "RunInference invocation failed"
        );
        self.push(run_inference_server_message::Message::InvocationEnd(
            RunInferenceInvocationEnd {
                invocation_id: invocation_id.to_owned(),
                error: Some(RunInferenceInvocationError {
                    code: 2,
                    message: message.to_owned(),
                }),
            },
        ))
        .await;
    }

    async fn push(&self, message: run_inference_server_message::Message) {
        let payload = RunInferenceServerMessage {
            message: Some(message),
        }
        .encode_to_vec();
        self.push_frame(Bytes::from(payload), false).await;
    }

    /// Appends one frame; the sequence number is assigned under the same lock so
    /// concurrent provider events cannot collide on it.
    async fn push_frame(&self, payload: Bytes, eof: bool) {
        let frame = {
            let mut state = self.state.lock().await;
            let seqno = if eof { -1 } else { state.frames.len() as i64 };
            let frame = Arc::new(Frame { seqno, payload, eof });
            state.frames.push(frame.clone());
            frame
        };
        self.changed.notify_waiters();
        let _ = self.feed.send(frame);
    }

    /// Replays every frame produced so far and subscribes to new ones.
    pub async fn subscribe(&self) -> (Vec<Arc<Frame>>, broadcast::Receiver<Arc<Frame>>) {
        let receiver = self.feed.subscribe();
        let frames = self.state.lock().await.frames.clone();
        (frames, receiver)
    }

    /// Returns the frames the client has not consumed yet, waiting for new ones.
    pub async fn poll(&self, start: bool) -> Vec<Arc<Frame>> {
        let deadline = tokio::time::Instant::now() + POLL_WAIT;
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut state = self.state.lock().await;
                if start {
                    state.poll_cursor = 0;
                }
                if state.frames.len() > state.poll_cursor {
                    let batch = state.frames[state.poll_cursor..].to_vec();
                    state.poll_cursor = state.frames.len();
                    return batch;
                }
                if state.finished {
                    return Vec::new();
                }
            }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                return Vec::new();
            }
        }
    }

    /// Heartbeat frames keep idle runs visible to the client's poll loop.
    pub async fn heartbeat(&self) {
        self.push(run_inference_server_message::Message::Heartbeat(
            RunInferenceHeartbeat {
                accepts_speculative_invocations: false,
                accepts_compressed_speculative_invocations: false,
            },
        ))
        .await;
    }
}
