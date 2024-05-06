use crate::error::AppResult;
use askama_axum::Template;
use axum::{
    extract::{Path, State},
    Router,
};
use chrono::{DateTime, Utc};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use shuttle_persist::{PersistError, PersistInstance};
use std::cmp::Ordering;
use tower_http::services::{ServeDir, ServeFile};

pub mod api;
pub mod error;

#[derive(Clone, Debug)]
pub struct AppState {
    pub persist: PersistInstance,
}

pub type Nf64 = NotNan<f64>;

// https://github.com/serde-rs/serde/issues/2578
mod serde_ordering {
    use std::cmp::Ordering;

    use serde::{
        de::{Error, Unexpected},
        Deserialize, Deserializer, Serialize, Serializer,
    };

    pub fn serialize<S: Serializer>(ordering: &Ordering, serializer: S) -> Result<S::Ok, S::Error> {
        (*ordering as i8).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Ordering, D::Error>
    where
        D: Deserializer<'de>,
    {
        i8::deserialize(deserializer).and_then(|i| match i {
            -1 => Ok(Ordering::Less),
            0 => Ok(Ordering::Equal),
            1 => Ok(Ordering::Greater),
            _ => Err(D::Error::invalid_value(
                Unexpected::Signed(i.into()),
                &"an integer in the range of -1i8..=1i8",
            )),
        })
    }
}

/// The state, in terms of what has been submitted, for a comparison
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ComparisonState {
    /// Alice has submitted something
    Value(Nf64),
    /// Alice and Bob have submitted
    Result(#[serde(with = "serde_ordering")] Ordering),
}

/// The information we store in our DB about each comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    /// Name of comparison
    pub name: String,
    /// Time of Alice's submission, currently not used except to keep history
    pub time: DateTime<Utc>,
    /// State of comparison
    pub state: ComparisonState,
}

pub fn err_is_no_such_id(err: &PersistError) -> bool {
    matches!(err, PersistError::InvalidKey | PersistError::Open(_))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

async fn index() -> IndexTemplate {
    IndexTemplate
}

#[derive(Debug, Clone)]
enum BobState {
    /// The ID doesn't exist
    NoSuchId,
    /// Bob has submitted a value already
    IdComparedAlready { id: String },
    /// Bob needs to submit something
    BobMain { name: String, id: String },
}

#[derive(Template)]
#[template(path = "bob.html")]
struct BobTemplate {
    state: BobState,
}

async fn bob(State(state): State<AppState>, Path(id): Path<String>) -> AppResult<BobTemplate> {
    let data = match state.persist.load::<Comparison>(&id) {
        Ok(d) => d,
        Err(e) if err_is_no_such_id(&e) => {
            return Ok(BobTemplate {
                state: BobState::NoSuchId,
            })
        }
        Err(e) => Err(e)?,
    };

    if matches!(data.state, ComparisonState::Result(_)) {
        return Ok(BobTemplate {
            state: BobState::IdComparedAlready { id },
        });
    }

    Ok(BobTemplate {
        state: BobState::BobMain {
            id,
            name: data.name,
        },
    })
}

#[derive(Debug, Clone)]
enum ViewState {
    /// The ID doesn't exist
    NoSuchId,
    /// Bob has not yet submitted a value
    IdNotCompared { name: String, id: String },
    /// Comparison finalized
    ViewMain { name: String, result: Ordering },
}

#[derive(Template)]
#[template(path = "view.html")]
struct ViewTemplate {
    state: ViewState,
}

async fn view(State(state): State<AppState>, Path(id): Path<String>) -> AppResult<ViewTemplate> {
    let data = match state.persist.load::<Comparison>(&id) {
        Ok(d) => d,
        Err(e) if err_is_no_such_id(&e) => {
            return Ok(ViewTemplate {
                state: ViewState::NoSuchId,
            })
        }
        Err(e) => Err(e)?,
    };

    let ComparisonState::Result(ord) = data.state else {
        return Ok(ViewTemplate {
            state: ViewState::IdNotCompared {
                name: data.name,
                id,
            },
        });
    };

    Ok(ViewTemplate {
        state: ViewState::ViewMain {
            name: data.name,
            result: ord,
        },
    })
}

pub fn app(persist: PersistInstance) -> Router {
    use axum::routing::get;

    let state = AppState { persist };

    let html_routes = Router::new()
        .route("/", get(index))
        .route("/bob/:id", get(bob))
        .route("/view/:id", get(view));

    let api_routes = Router::new().nest("/api", api::app());

    Router::new()
        .merge(html_routes)
        .merge(api_routes)
        .fallback_service(ServeDir::new("static").fallback(ServeFile::new("static/404.html")))
        .with_state(state)
}
