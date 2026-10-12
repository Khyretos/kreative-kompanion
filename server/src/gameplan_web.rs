//! M6-11: POST /api/projects/{id}/game-plan. A game described in words becomes a pending proposal (code tasks and
//! asset tasks as a KG-02 plan); nothing is created until the user applies it through POST /api/plans/{id}/apply.
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState, api,
    auth::User,
    error::{ApiError, ApiResult},
    events::Event,
    gameplan::{self, GamePlan, LibAsset, WfInfo},
    llm,
    planner::{self, Issue, Limits},
};

#[derive(Deserialize)]
pub struct DescribeBody {
    description: String,
}

/// One planning run on the user's orchestrator model, schema-constrained, with the rule-fixing rounds.
async fn plan_with_model(s: &AppState, user_id: &str, system: &str, user: &str, library: &[LibAsset], workflows: &[WfInfo]) -> ApiResult<(GamePlan, Vec<Issue>)> {
    let role = api::user_role(s, user_id, "orchestrator").await?.ok_or_else(|| ApiError::BadRequest("Set the orchestrator model first.".into()))?;
    let provider = s.config.provider(&role.provider_id).ok_or_else(|| ApiError::BadRequest(format!("The provider {} is not configured.", role.provider_id)))?;
    let (p, model) = crate::effort::apply(provider, &role.model_id, crate::effort::Effort::default());
    let schema = gameplan::schema();
    let ask = |messages: Vec<Value>| {
        let (http, p, model, schema) = (s.http.clone(), p.clone(), model.clone(), schema.clone());
        async move {
            // A server without schema support answers 400: ask again without it (the parser and the checks still apply).
            let first = llm::chat_json(&http, &p, &model, &messages, &schema).await;
            let answer = match first {
                Ok(a) => a,
                Err(_) => llm::chat_with_tools_full(&http, &p, &model, &messages, &json!([])).await.map_err(|e| format!("{e:#}"))?,
            };
            Ok::<String, String>(answer.0["content"].as_str().unwrap_or("").to_string())
        }
    };
    gameplan::generate(ask, system, user, &Limits::default(), library, workflows).await.map_err(|e| ApiError::BadRequest(format!("Planning failed: {e}")))
}

/// POST /api/projects/{id}/game-plan: the game in words becomes a pending proposal.
pub async fn propose(State(s): State<AppState>, Extension(u): Extension<User>, Path(project): Path<String>, Json(b): Json<DescribeBody>) -> ApiResult<(StatusCode, Json<Value>)> {
    let description = b.description.trim();
    if description.len() < 10 {
        return Err(ApiError::BadRequest("Describe the game in a sentence or two.".into()));
    }
    let row: Option<(String, String)> = sqlx::query_as("SELECT name, description FROM projects WHERE id = ? AND user_id = ?").bind(&project).bind(&u.id).fetch_optional(&s.db).await?;
    let (name, about) = row.ok_or(ApiError::NotFound)?;
    let dev = crate::devsettings_web::brief_of(&s.db, &project, &u.id).await;
    let state = crate::statesum::summary_of(&s.db, &project, &u.id).await;
    let library = gameplan::library_of(&s.db, &u.id, &project).await;
    let workflows = {
        let loaded: Vec<(String, crate::studio::workflow::Workflow)> = crate::studio::workflow::load_all(&s.config.studio.workflows_dir).into_iter().filter_map(|(n, r)| r.ok().map(|w| (n, w))).collect();
        let mut infos = gameplan::wf_infos(&loaded);
        infos.extend(gameplan::audio_infos());
        infos
    };
    let user_msg = gameplan::user_message(&name, &about, &dev, &state, &library, &workflows, description);
    let system = gameplan::system_text(&Limits::default());
    let (game, left) = plan_with_model(&s, &u.id, &system, &user_msg, &library, &workflows).await?;
    let id = planner::store_proposal(&s.db, &u.id, &project, None, "plan", description, &gameplan::to_plan(&game)).await?;
    s.bus.send(&u.id, Event::Changed { what: "tasks", machine_id: None });
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "kind": "plan", "gamePlan": game, "plan": gameplan::to_plan(&game), "issues": left }))))
}
