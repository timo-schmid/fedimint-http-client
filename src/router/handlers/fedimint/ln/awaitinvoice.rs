use anyhow::anyhow;
use axum::{extract::State, http::StatusCode, Json};
use fedimint_ln_client::{LightningClientModule, LnReceiveState};
use futures::StreamExt;
use tracing::info;

use crate::{
    error::AppError,
    state::AppState,
    types::fedimint::{AwaitInvoiceRequest, InfoResponse},
    utils::get_note_summary,
};

#[axum_macros::debug_handler]
pub async fn handle_awaitinvoice(
    State(state): State<AppState>,
    Json(req): Json<AwaitInvoiceRequest>,
) -> Result<Json<InfoResponse>, AppError> {
    let lightning_module = &state.fm.get_first_module::<LightningClientModule>();
    let mut updates = lightning_module
        .subscribe_ln_receive(req.operation_id)
        .await?
        .into_stream();
    while let Some(update) = updates.next().await {
        match update {
            LnReceiveState::Claimed => {
                return Ok(Json(get_note_summary(&state.fm).await?));
            }
            LnReceiveState::Canceled { reason } => {
                return Err(AppError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    anyhow!(reason),
                ))
            }
            _ => {}
        }

        info!("Update: {update:?}");
    }

    Err(AppError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        anyhow!("Unexpected end of stream"),
    ))
}
