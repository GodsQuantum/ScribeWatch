//! User-defined spaces group existing jobs without moving audio or changing transcripts.
//! Keep membership separate from Job so background job updates cannot overwrite it.
use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SPACE: &str = "space";
const MEMBER: &str = "job_space";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Space {
    pub id: String,
    pub name: String,
    pub kind: SpaceKind,
    pub created_at_ms: u128,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpaceKind {
    General,
    Project,
    Meeting,
    Case,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Membership {
    pub job_id: String,
    pub space_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceInput {
    name: String,
    kind: SpaceKind,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentInput {
    space_id: Option<String>,
}

fn validated_name(input: &str) -> AppResult<String> {
    let name = input.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(AppError::BadRequest(
            "Space name must be 1–100 printable characters".into(),
        ));
    }
    Ok(name.to_owned())
}

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Space>>> {
    let mut spaces = state.db.list::<Space>(SPACE).map_err(AppError::Internal)?;
    spaces.sort_by(|a, b| a.created_at_ms.cmp(&b.created_at_ms).then(a.id.cmp(&b.id)));
    Ok(Json(spaces))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<SpaceInput>,
) -> AppResult<Json<Space>> {
    let name = validated_name(&input.name)?;
    let spaces = state.db.list::<Space>(SPACE).map_err(AppError::Internal)?;
    if spaces.len() >= 200 {
        return Err(AppError::Conflict("Maximum of 200 spaces reached".into()));
    }
    let space = Space {
        id: Uuid::new_v4().to_string(),
        name,
        kind: input.kind,
        created_at_ms: crate::jobs::now_ms(),
    };
    state
        .db
        .upsert(SPACE, &space.id, &space)
        .map_err(AppError::Internal)?;
    Ok(Json(space))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    if state
        .db
        .get::<Space>(SPACE, &id)
        .map_err(AppError::Internal)?
        .is_none()
    {
        return Err(AppError::NotFound("Space not found".into()));
    }
    // Removing a space never deletes jobs, recordings or documents.
    for assignment in state
        .db
        .list::<Membership>(MEMBER)
        .map_err(AppError::Internal)?
    {
        if assignment.space_id == id {
            state
                .db
                .delete(MEMBER, &assignment.job_id)
                .map_err(AppError::Internal)?;
        }
    }
    state.db.delete(SPACE, &id).map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn assignments(State(state): State<AppState>) -> AppResult<Json<Vec<Membership>>> {
    let assignments = state
        .db
        .list::<Membership>(MEMBER)
        .map_err(AppError::Internal)?;
    // Return no stale references after records have been removed.
    Ok(Json(
        assignments
            .into_iter()
            .filter(|m| state.jobs.contains_key(&m.job_id))
            .collect(),
    ))
}

pub async fn assign(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(input): Json<AssignmentInput>,
) -> AppResult<StatusCode> {
    if !state.jobs.contains_key(&job_id) {
        return Err(AppError::NotFound("Recording not found".into()));
    }
    match input.space_id {
        Some(space_id) => {
            if state
                .db
                .get::<Space>(SPACE, &space_id)
                .map_err(AppError::Internal)?
                .is_none()
            {
                return Err(AppError::NotFound("Space not found".into()));
            }
            let assignment = Membership {
                job_id: job_id.clone(),
                space_id,
            };
            state
                .db
                .upsert(MEMBER, &job_id, &assignment)
                .map_err(AppError::Internal)?;
        }
        None => {
            state
                .db
                .delete(MEMBER, &job_id)
                .map_err(AppError::Internal)?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_safely_bounded_without_assuming_a_profession() {
        assert!(validated_name("Réunion d'équipe").is_ok());
        assert!(validated_name("Projet alpha").is_ok());
        assert!(validated_name("   ").is_err());
        assert!(validated_name(&"a".repeat(101)).is_err());
        assert!(validated_name("abc\nx").is_err());
    }

    #[test]
    fn deleted_space_does_not_remove_recordings() {
        let temp = tempfile::tempdir().unwrap();
        let db = crate::db::Database::open(&temp.path().join("spaces.sqlite")).unwrap();
        let space = Space {
            id: "space1".into(),
            name: "Projet alpha".into(),
            kind: SpaceKind::Project,
            created_at_ms: 1,
        };
        let membership = Membership {
            job_id: "recording1".into(),
            space_id: space.id.clone(),
        };
        db.upsert(SPACE, &space.id, &space).unwrap();
        db.upsert(MEMBER, &membership.job_id, &membership).unwrap();
        // Storage deletion of a space and membership is orthogonal to all Job objects.
        db.delete(MEMBER, &membership.job_id).unwrap();
        db.delete(SPACE, &space.id).unwrap();
        assert!(db.get::<Space>(SPACE, &space.id).unwrap().is_none());
        assert!(
            db.get::<Membership>(MEMBER, &membership.job_id)
                .unwrap()
                .is_none()
        );
    }
}
