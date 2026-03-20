use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use tracing::error;

use crate::server::AppState;
use crate::storage::MessagePart;

#[derive(Debug, Deserialize)]
pub struct MessagePathParams {
    id: String,
    message_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PartPathParams {
    id: String,
    message_id: String,
    part_id: String,
}

pub async fn get_message(
    State(state): State<AppState>,
    Path(params): Path<MessagePathParams>,
) -> Result<Json<crate::storage::MessageWithParts>, StatusCode> {
    match state
        .message_store
        .get_message(&params.id, &params.message_id)
        .await
    {
        Ok(Some(message)) => Ok(Json(message)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get message: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_message(
    State(state): State<AppState>,
    Path(params): Path<MessagePathParams>,
) -> Result<Json<bool>, StatusCode> {
    match state
        .message_store
        .get_message(&params.id, &params.message_id)
        .await
    {
        Ok(Some(_)) => {
            match state
                .message_store
                .remove(&params.id, &params.message_id)
                .await
            {
                Ok(_) => Ok(Json(true)),
                Err(e) => {
                    error!("Failed to delete message: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to check message existence: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_part(
    State(state): State<AppState>,
    Path(params): Path<PartPathParams>,
) -> Result<Json<bool>, StatusCode> {
    match state
        .message_store
        .get_message(&params.id, &params.message_id)
        .await
    {
        Ok(Some(_)) => {
            match state
                .message_store
                .remove_part(&params.id, &params.message_id, &params.part_id)
                .await
            {
                Ok(result) => Ok(Json(result)),
                Err(e) => {
                    error!("Failed to delete part: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to check message existence: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_part(
    State(state): State<AppState>,
    Path(params): Path<PartPathParams>,
    Json(body): Json<MessagePart>,
) -> Result<Json<MessagePart>, StatusCode> {
    if body.id() != params.part_id {
        error!(
            "Part mismatch: body.id='{}' vs part_id='{}'",
            body.id(),
            params.part_id
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    match state
        .message_store
        .get_message(&params.id, &params.message_id)
        .await
    {
        Ok(Some(_)) => {
            match state
                .message_store
                .update_part(&params.id, &params.message_id, body)
                .await
            {
                Ok(part) => Ok(Json(part)),
                Err(e) => {
                    error!("Failed to update part: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to check message existence: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}