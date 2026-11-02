//! Serves the Cursor `InferenceService` run endpoints locally.
//!
//! `RunInference` (HTTP/2 bi-di), `RunInferenceSSE` and `RunInferencePoll` all
//! attach to one `InferenceRegistry` session keyed by the Bidi request id, so the
//! same run can move between transports without replaying client input.
use std::sync::Arc;

use axum::{
    body::{to_bytes, Body, Bytes},
    extract::Extension,
    http::{header, HeaderValue, Request, Response, StatusCode},
};
use bytes::BytesMut;
use futures_util::StreamExt;
use prost::Message;

use crate::{
    cursor::{
        inference::{InferenceRegistry, Session},
        protocol::{
            connect,
            inference::{BidiPollRequest, BidiPollResponse, RunInferenceClientMessage},
            proto::aiserver::v1 as ai,
        },
    },
    Error, Result,
};

/// `RunInferenceSSE`: server-streaming subscription for one run.
pub async fn sse(
    Extension(inference): Extension<InferenceRegistry>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let request_id = request_id(&buffered(request).await?)?;
    let session = inference.session(&request_id).await;
    Ok(run_stream(session))
}

/// `RunInferencePoll`: pulls the next sequence of run frames.
pub async fn poll(
    Extension(inference): Extension<InferenceRegistry>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let body = buffered(request).await?;
    let poll: BidiPollRequest = connect::decode_unary(&body)?;
    let request_id = poll
        .request_id
        .as_ref()
        .map(|id| id.request_id.clone())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| Error::Protocol("BidiPollRequest request_id is required".into()))?;
    let session = inference.session(&request_id).await;
    let frames = session.poll(poll.start_request.unwrap_or(false)).await;
    let mut body = BytesMut::new();
    for frame in frames {
        let response = BidiPollResponse {
            seqno: frame.seqno,
            data: if frame.eof {
                String::new()
            } else {
                hex::encode(&frame.payload)
            },
            eof: frame.eof.then_some(true),
        };
        body.extend_from_slice(&connect::encode_message(&response)?);
    }
    body.extend_from_slice(&connect::encode_end_stream());
    let mut response = Response::new(Body::from(body.freeze()));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, connect_content_type());
    response
        .headers_mut()
        .insert("connect-protocol-version", HeaderValue::from_static("1"));
    Ok(response)
}

/// `RunInference`: HTTP/2 bi-di stream. The first frame names the run, the rest
/// are client messages appended to the same session.
///
/// The response must start as soon as the run is known, so the remaining client
/// frames are drained by a background task while the response streams.
pub async fn bidi(
    Extension(inference): Extension<InferenceRegistry>,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let (_, body) = request.into_parts();
    let mut reader = body.into_data_stream();
    let mut buffer = FrameBuffer::default();
    let session = loop {
        let Some(chunk) = reader.next().await else {
            return Err(Error::Protocol(
                "RunInference stream carried no request id".into(),
            ));
        };
        let chunk = chunk
            .map_err(|error| Error::Protocol(format!("cannot read RunInference body: {error}")))?;
        let payloads = buffer.push(&chunk);
        let Some((first, rest)) = payloads.split_first() else {
            continue;
        };
        let created = inference.session(&request_id(first)?).await;
        for payload in rest {
            created
                .clone()
                .append(RunInferenceClientMessage::decode(payload.as_ref())?)
                .await?;
        }
        break created;
    };
    let response = run_stream(session.clone());
    tokio::spawn(async move {
        while let Some(chunk) = reader.next().await {
            let Ok(chunk) = chunk else {
                break;
            };
            for payload in buffer.push(&chunk) {
                match RunInferenceClientMessage::decode(payload.as_ref()) {
                    Ok(message) => {
                        if let Err(error) = session.clone().append(message).await {
                            tracing::warn!(%error, "rejecting RunInference client message");
                            return;
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "undecodable RunInference client message");
                        return;
                    }
                }
            }
        }
    });
    Ok(response)
}

fn run_stream(session: Arc<Session>) -> Response<Body> {
    let stream = async_stream::stream! {
        let (replayed, mut receiver) = session.subscribe().await;
        let mut last = -1i64;
        for frame in replayed {
            last = frame.seqno;
            if frame.eof {
                yield Ok::<_, std::convert::Infallible>(connect::encode_end_stream());
                return;
            }
            yield Ok::<_, std::convert::Infallible>(connect::encode_message_payload(&frame.payload));
        }
        loop {
            match receiver.recv().await {
                Ok(frame) => {
                    if frame.eof {
                        yield Ok::<_, std::convert::Infallible>(connect::encode_end_stream());
                        return;
                    }
                    if frame.seqno <= last {
                        continue;
                    }
                    last = frame.seqno;
                    yield Ok::<_, std::convert::Infallible>(connect::encode_message_payload(&frame.payload));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            }
        }
    };
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, connect_content_type());
    response
        .headers_mut()
        .insert("connect-protocol-version", HeaderValue::from_static("1"));
    response
}

fn connect_content_type() -> HeaderValue {
    HeaderValue::from_static("application/connect+proto")
}

fn request_id(body: &[u8]) -> Result<String> {
    let payload = match connect::decode_frames(body) {
        Ok(frames) => frames
            .into_iter()
            .next()
            .map(|(_, payload)| payload)
            .ok_or_else(|| Error::Protocol("RunInference stream carried no frames".into()))?,
        Err(_) => Bytes::copy_from_slice(body),
    };
    let id: ai::BidiRequestId = Message::decode(payload.as_ref())?;
    if id.request_id.is_empty() {
        return Err(Error::Protocol("RunInference BidiRequestId is empty".into()));
    }
    Ok(id.request_id)
}

async fn buffered(request: Request<Body>) -> Result<Bytes> {
    to_bytes(request.into_body(), usize::MAX)
        .await
        .map_err(|error| Error::Protocol(format!("cannot read request body: {error}")))
}

/// Splits a byte stream into Connect payloads.
#[derive(Default)]
struct FrameBuffer {
    buffer: BytesMut,
}

impl FrameBuffer {
    fn push(&mut self, chunk: &[u8]) -> Vec<Bytes> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();
        loop {
            if self.buffer.len() < 5 {
                break;
            }
            let length = u32::from_be_bytes([
                self.buffer[1],
                self.buffer[2],
                self.buffer[3],
                self.buffer[4],
            ]) as usize;
            if self.buffer.len() < 5 + length {
                break;
            }
            let mut frame = self.buffer.split_to(5 + length);
            let payload = frame.split_off(5);
            frames.push(payload.freeze());
        }
        frames
    }
}
