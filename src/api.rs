use crate::{
    err_is_no_such_id,
    error::{ApiError, ApiResult},
    AppState, Comparison, ComparisonState, Nf64,
};
use axum::{extract::State, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shuttle_persist::PersistInstance;
use uuid::Uuid;

/// Request for [`store`]
#[derive(Deserialize)]
struct StoreReq {
    /// Name of comparison
    name: String,
    /// Value Alice submits
    value: Nf64,
}

/// Response for [`store`]
#[derive(Serialize)]
struct StoreRes {
    /// Id of the newly created comparison
    id: String,
}

/// Alice creates a new comparison
async fn store(
    State(state): State<AppState>,
    Json(req): Json<StoreReq>,
) -> ApiResult<Json<StoreRes>> {
    let id = Uuid::new_v4().to_string();

    state.persist.save(
        &id,
        Comparison {
            name: req.name,
            time: Utc::now(),
            state: ComparisonState::Value(req.value),
        },
    )?;

    Ok(Json(StoreRes { id }))
}

/// Request for [`compare`]
#[derive(Deserialize)]
struct CompareReq {
    /// Id of comparison
    id: String,
    /// Value Bob submits
    value: Nf64,
}

/// Response for [`compare`]
#[derive(Serialize)]
struct CompareRes {
    // TODO: the only reason we have this is because I can't figure out how to get serde_json to serialize an empty object
    ok: (),
}

/// Try loading the comparison from our DB, and return [`ApiError::NoSuchId`] if `key` doesn't exist.
fn persist_load(persist: &PersistInstance, key: &str) -> ApiResult<Comparison> {
    persist.load(key).map_err(|e| {
        if err_is_no_such_id(&e) {
            ApiError::NoSuchId
        } else {
            e.into()
        }
    })
}

/// Bob submits his number
async fn compare(
    State(state): State<AppState>,
    Json(req): Json<CompareReq>,
) -> ApiResult<Json<CompareRes>> {
    let mut data = persist_load(&state.persist, &req.id)?;
    let ComparisonState::Value(orig_value) = data.state else {
        return Err(ApiError::IdComparedAlready);
    };

    data.state = ComparisonState::Result(orig_value.cmp(&req.value));

    state.persist.save(&req.id, data)?;

    Ok(Json(CompareRes { ok: () }))
}

/// Request of [`result`]
#[derive(Deserialize)]
struct ResultReq {
    id: String,
}

/// Response of [`result`]
#[derive(Serialize)]
struct ResultRes {
    name: String,
    result: i8,
}

/// Get the result of a comparison
async fn result(
    State(state): State<AppState>,
    Json(req): Json<ResultReq>,
) -> ApiResult<Json<ResultRes>> {
    let data = persist_load(&state.persist, &req.id)?;
    let ComparisonState::Result(ord) = data.state else {
        return Err(ApiError::IdNotCompared);
    };

    Ok(Json(ResultRes {
        name: data.name,
        result: ord as i8,
    }))
}

pub fn app() -> Router<AppState> {
    use axum::routing::post;
    use tower_http::cors::{Any as CorsAny, CorsLayer};

    let cors = CorsLayer::new()
        .allow_methods(CorsAny)
        .allow_origin(CorsAny)
        .allow_headers(CorsAny);

    Router::new()
        .route("/store", post(store))
        .route("/compare", post(compare))
        .route("/result", post(result))
        .layer(cors)
}
