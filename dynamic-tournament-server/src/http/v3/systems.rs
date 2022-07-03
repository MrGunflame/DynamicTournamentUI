use dynamic_tournament_generator::options::TournamentOptions;
use hyper::{Body, Method, Response, StatusCode};

use crate::http::{Request, RequestUri};
use crate::{method, Error, StatusCodeError};

use dynamic_tournament_api::v3::id::SystemId;
use dynamic_tournament_api::v3::systems::{System, SystemOverview};
use dynamic_tournament_generator::{EntrantScore, SingleElimination};

pub async fn route(req: Request, mut uri: RequestUri<'_>) -> Result<Response<Body>, Error> {
    match uri.take() {
        None => method!(req, {
            Method::GET => list(req).await,
        }),
        Some(part) => {
            let id = part.parse()?;

            method!(req, {
                Method::GET => get(req, id).await,
            })
        }
    }
}

async fn list(_req: Request) -> Result<Response<Body>, Error> {
    // Hardcoded for now.
    let systems = [
        SystemOverview {
            id: SystemId(1),
            name: "Single Elimination".into(),
        },
        SystemOverview {
            id: SystemId(2),
            name: "Double Elimination".into(),
        },
    ];

    let body = serde_json::to_vec(&systems)?;

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();

    Ok(resp)
}

async fn get(_req: Request, id: SystemId) -> Result<Response<Body>, Error> {
    let system = match id.as_ref() {
        1 => Some(System {
            id: SystemId(1),
            name: "Single Elimination".into(),
            options: SingleElimination::<u8, EntrantScore<u8>>::options(),
        }),
        2 => Some(System {
            id: SystemId(2),
            name: "Double Elimination".into(),
            options: TournamentOptions::default(),
        }),
        _ => None,
    };

    match system {
        Some(system) => {
            let body = serde_json::to_vec(&system)?;

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap())
        }
        None => Err(StatusCodeError::not_found().into()),
    }
}
