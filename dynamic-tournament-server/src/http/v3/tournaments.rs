mod brackets;
mod entrants;
mod roles;

use dynamic_tournament_api::v3::id::TournamentId;
use hyper::{Body, Method, Response, StatusCode};

use crate::method;
use crate::{
    http::{Request, RequestUri},
    Error, State, StatusCodeError,
};

pub async fn route(
    req: Request,
    mut uri: RequestUri<'_>,
    state: State,
) -> Result<Response<Body>, Error> {
    match uri.take() {
        None => method!(req, {
            Method::GET => list(req, state).await,
            Method::POST => create(req, state).await,
        }),
        Some(part) => {
            let id = part.parse()?;

            match uri.take_str() {
                Some("entrants") => entrants::route(req, uri, state, id).await,
                Some("brackets") => brackets::route(req, uri, state, id).await,
                Some("roles") => roles::route(req, uri, state, id).await,
                None => method!(req, {
                    Method::GET => get(req, state, id).await,
                }),
                Some(_) => Err(StatusCodeError::not_found().into()),
            }
        }
    }
}

async fn list(_req: Request, state: State) -> Result<Response<Body>, Error> {
    let tournaments = state.store.list_tournaments().await?;

    let body = serde_json::to_vec(&tournaments)?;

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap();

    Ok(resp)
}

async fn get(_req: Request, state: State, id: TournamentId) -> Result<Response<Body>, Error> {
    let tournament = state.store.get_tournament(id).await?;

    let tournament = tournament.ok_or_else(StatusCodeError::not_found)?;

    Ok(Response::new(Body::from(serde_json::to_vec(&tournament)?)))
}

async fn create(req: Request, state: State) -> Result<Response<Body>, Error> {
    if !state.is_authenticated(&req) {
        return Err(StatusCodeError::unauthorized().into());
    }

    let tournament = req.json().await?;

    let id = state.store.insert_tournament(&tournament).await?;

    Ok(Response::new(Body::from(id.to_string())))
}
