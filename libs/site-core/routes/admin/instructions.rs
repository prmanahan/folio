use crate::error::AppError;
use crate::models::instructions::{self, AiInstruction, AiInstructionInput};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

async fn list_instructions(
    State(state): State<DbState>,
) -> Result<Json<Vec<AiInstruction>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = instructions::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_instruction(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<AiInstruction>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = instructions::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Instruction {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_instruction(
    State(state): State<DbState>,
    Json(input): Json<AiInstructionInput>,
) -> Result<(StatusCode, Json<AiInstruction>), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = instructions::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_instruction(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<AiInstructionInput>,
) -> Result<Json<AiInstruction>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = instructions::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Instruction {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_instruction(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    instructions::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route(
            "/api/admin/instructions",
            get(list_instructions).post(create_instruction),
        )
        .route(
            "/api/admin/instructions/{id}",
            get(get_instruction)
                .put(update_instruction)
                .delete(delete_instruction),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::instructions;

    fn make_input(instr: &str, priority: i64) -> instructions::AiInstructionInput {
        instructions::AiInstructionInput {
            instruction_type: "honesty".to_string(),
            instruction: instr.to_string(),
            priority,
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = instructions::create(&conn, &make_input("Always be honest", 10)).unwrap();
        let b = instructions::create(&conn, &make_input("Never fabricate experience", 9)).unwrap();

        assert_eq!(a.instruction, "Always be honest");
        assert_eq!(a.priority, 10);

        let all = instructions::list_all(&conn).unwrap();
        assert!(all.iter().any(|i| i.id == a.id));
        assert!(all.iter().any(|i| i.id == b.id));

        // list is ordered by priority DESC
        let first = &all[0];
        assert_eq!(first.priority, 10);

        let fetched = instructions::get_by_id(&conn, b.id).unwrap();
        assert_eq!(fetched.instruction_type, "honesty");

        let updated =
            instructions::update(&conn, a.id, &make_input("Always be honest and direct", 10))
                .unwrap();
        assert_eq!(updated.instruction, "Always be honest and direct");

        instructions::delete(&conn, b.id).unwrap();
        let result = instructions::get_by_id(&conn, b.id);
        assert!(result.is_err());

        let remaining = instructions::list_all(&conn).unwrap();
        assert!(!remaining.iter().any(|i| i.id == b.id));
    }
}
