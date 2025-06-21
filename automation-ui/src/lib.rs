use automation_ai::AIAutomationEngine;
use automation_integration::ACAutomationIntegration;
use axum::http::Method;
use axum::{
    Form, Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::Any;
use tower_http::{cors::CorsLayer, services::ServeDir};

pub struct AppState {
    pub integration: ACAutomationIntegration,
    pub ai_engine: Option<AIAutomationEngine>,
}

pub type SharedState = Arc<Mutex<AppState>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessACRequest {
    pub website_url: String,
    pub test_scenario: String,
    #[serde(default)]
    pub test_title: Option<String>,
    #[serde(default)]
    pub execute_immediately: Option<bool>,
    #[serde(default)]
    pub use_ai: Option<bool>,
    #[serde(default)]
    pub openrouter_api_key: Option<String>,
    #[serde(default)]
    pub ai_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessACResponse {
    pub success: bool,
    pub message: String,
    pub workflow_id: Option<String>,
    pub workflow: Option<automation_api::AutomationWorkflow>,
    pub mcp_script: Option<String>,
    pub execution_report: Option<automation_browser::ExecutionReport>,
    pub ai_used: bool,
    pub ai_plan: Option<automation_ai::AIExecutionPlan>,
    pub ai_reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListResponse {
    pub workflows: Vec<WorkflowSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub step_count: usize,
    pub tags: Vec<String>,
}

pub fn create_app(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_credentials(false);

    Router::new()
        .route("/", get(home_page))
        .route(
            "/api/process",
            post(process_acceptance_criteria).options(options_handler),
        )
        .route(
            "/api/process-form",
            post(process_acceptance_criteria_form).options(options_handler),
        )
        .route(
            "/api/workflows",
            get(get_workflows).options(options_handler),
        )
        .route(
            "/api/execute/:workflow_id",
            post(execute_workflow).options(options_handler),
        )
        .route(
            "/api/script/:workflow_id",
            get(get_script).options(options_handler),
        )
        .nest_service("/static", ServeDir::new("automation-ui/static"))
        .layer(cors)
        .fallback(options_handler)
        .with_state(state)
}

pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let integration = ACAutomationIntegration::new()?;
    let app_state = AppState {
        integration,
        ai_engine: None,
    };
    let state = Arc::new(Mutex::new(app_state));

    let app = create_app(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!(
        "🌐 AC Automation UI Server running on http://localhost:{}",
        port
    );
    println!("📋 Open your browser to start converting acceptance criteria to automation!");
    println!("🤖 AI-powered automation available with OpenRouter API key!");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn home_page() -> impl IntoResponse {
    match std::fs::read_to_string("automation-ui/templates/index.html") {
        Ok(content) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(content),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Html(format!("<h1>Error loading template: {}</h1>", e)),
        ),
    }
}

async fn options_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (
                axum::http::header::ACCESS_CONTROL_ALLOW_METHODS,
                "GET, POST, PUT, DELETE, OPTIONS",
            ),
            (
                axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
                "Content-Type, Authorization, Accept",
            ),
            (axum::http::header::ACCESS_CONTROL_MAX_AGE, "86400"),
        ],
        "",
    )
}

async fn process_acceptance_criteria(
    State(state): State<SharedState>,
    Json(request): Json<ProcessACRequest>,
) -> impl IntoResponse {
    let title = request
        .test_title
        .clone()
        .unwrap_or_else(|| format!("Test for {}", request.website_url));
    println!("📝 Received AC processing request: {}", title);
    let mut app_state = state.lock().await;

    // Create tags from website URL
    let tags: Vec<String> = vec![
        "web".to_string(),
        "automation".to_string(),
        if request.website_url.contains("facebook") {
            "facebook".to_string()
        } else if request.website_url.contains("google") {
            "google".to_string()
        } else if request.website_url.contains("github") {
            "github".to_string()
        } else {
            "website".to_string()
        },
    ];

    // Check if AI should be used
    let use_ai = request.use_ai.unwrap_or(false);

    if use_ai {
        // AI-powered processing
        if let Some(api_key) = &request.openrouter_api_key {
            // Initialize AI engine if not already done or if API key changed
            if app_state.ai_engine.is_none() {
                match AIAutomationEngine::new(api_key.clone(), request.ai_model.clone()) {
                    Ok(ai_engine) => {
                        app_state.ai_engine = Some(ai_engine);
                        println!("🤖 AI Engine initialized successfully");
                    }
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(ProcessACResponse {
                                success: false,
                                message: format!("Failed to initialize AI engine: {}", e),
                                workflow_id: None,
                                workflow: None,
                                mcp_script: None,
                                execution_report: None,
                                ai_used: false,
                                ai_plan: None,
                                ai_reasoning: None,
                            }),
                        );
                    }
                }
            }

            if let Some(ai_engine) = &mut app_state.ai_engine {
                // Create acceptance criteria for AI
                let criteria = create_acceptance_criteria_from_request(&request, &tags);

                match ai_engine
                    .analyze_and_execute_ac(&criteria, request.execute_immediately.unwrap_or(false))
                    .await
                {
                    Ok(ai_result) => {
                        return (
                            StatusCode::OK,
                            Json(ProcessACResponse {
                                success: true,
                                message: "Successfully processed with AI".to_string(),
                                workflow_id: Some(format!(
                                    "ai_{}",
                                    uuid::Uuid::new_v4().to_string()[..8].to_string()
                                )),
                                workflow: None, // AI creates its own plan format
                                mcp_script: Some(ai_result.mcp_script),
                                execution_report: ai_result.execution_report,
                                ai_used: true,
                                ai_plan: Some(ai_result.plan),
                                ai_reasoning: Some(ai_result.llm_reasoning),
                            }),
                        );
                    }
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ProcessACResponse {
                                success: false,
                                message: format!("AI processing failed: {}", e),
                                workflow_id: None,
                                workflow: None,
                                mcp_script: None,
                                execution_report: None,
                                ai_used: true,
                                ai_plan: None,
                                ai_reasoning: None,
                            }),
                        );
                    }
                }
            }
        } else {
            return (
                StatusCode::BAD_REQUEST,
                Json(ProcessACResponse {
                    success: false,
                    message: "OpenRouter API key required for AI processing".to_string(),
                    workflow_id: None,
                    workflow: None,
                    mcp_script: None,
                    execution_report: None,
                    ai_used: false,
                    ai_plan: None,
                    ai_reasoning: None,
                }),
            );
        }
    }

    // Standard processing (non-AI)
    let criteria_text = format!(
        "Scenario: {}\nGiven I navigate to \"{}\"\n{}",
        title,
        request.website_url,
        request
            .test_scenario
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with('-') {
                    format!("When {}", trimmed.trim_start_matches('-').trim())
                } else if !trimmed.is_empty() {
                    format!("And {}", trimmed)
                } else {
                    String::new()
                }
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    );

    match app_state
        .integration
        .full_ac_to_automation_pipeline(
            title,
            "Web Automation".to_string(),
            criteria_text,
            tags,
            request.execute_immediately.unwrap_or(false),
        )
        .await
    {
        Ok(result) => {
            let response = ProcessACResponse {
                success: true,
                message: "Successfully processed acceptance criteria".to_string(),
                workflow_id: Some(result.workflow.id.clone()),
                workflow: Some(result.workflow),
                mcp_script: Some(result.mcp_script),
                execution_report: result.execution_report,
                ai_used: false,
                ai_plan: None,
                ai_reasoning: None,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = ProcessACResponse {
                success: false,
                message: format!("Failed to process: {}", e),
                workflow_id: None,
                workflow: None,
                mcp_script: None,
                execution_report: None,
                ai_used: false,
                ai_plan: None,
                ai_reasoning: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

async fn process_acceptance_criteria_form(
    State(state): State<SharedState>,
    Form(form_data): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    println!("📝 Received form data: {:?}", form_data);

    let request = ProcessACRequest {
        website_url: form_data.get("website_url").cloned().unwrap_or_default(),
        test_scenario: form_data.get("test_scenario").cloned().unwrap_or_default(),
        test_title: form_data
            .get("test_title")
            .cloned()
            .filter(|s| !s.is_empty()),
        execute_immediately: form_data.get("execute_immediately").map(|_| true),
        use_ai: form_data.get("use_ai").map(|_| true),
        openrouter_api_key: form_data
            .get("openrouter_api_key")
            .cloned()
            .filter(|s| !s.is_empty()),
        ai_model: form_data.get("ai_model").cloned().filter(|s| !s.is_empty()),
    };

    // Forward to the JSON handler
    process_acceptance_criteria(State(state), Json(request)).await
}

fn create_acceptance_criteria_from_request(
    request: &ProcessACRequest,
    tags: &[String],
) -> automation_api::AcceptanceCriteria {
    let title = request
        .test_title
        .clone()
        .unwrap_or_else(|| format!("Test for {}", request.website_url));

    let criteria_text = format!(
        "Scenario: {}\nGiven I navigate to \"{}\"\n{}",
        title,
        request.website_url,
        request
            .test_scenario
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with('-') {
                    format!("When {}", trimmed.trim_start_matches('-').trim())
                } else if !trimmed.is_empty() {
                    format!("And {}", trimmed)
                } else {
                    String::new()
                }
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    );

    let mut given_steps = Vec::new();
    let mut when_steps = Vec::new();
    let mut then_steps = Vec::new();
    let mut scenario = title.clone();

    let lines: Vec<&str> = criteria_text.lines().collect();
    let mut current_step_type = "";

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("Scenario:") {
            scenario = trimmed.replace("Scenario:", "").trim().to_string();
        } else if trimmed.starts_with("Given") {
            current_step_type = "given";
            given_steps.push(trimmed.replace("Given", "").trim().to_string());
        } else if trimmed.starts_with("When") {
            current_step_type = "when";
            when_steps.push(trimmed.replace("When", "").trim().to_string());
        } else if trimmed.starts_with("Then") {
            current_step_type = "then";
            then_steps.push(trimmed.replace("Then", "").trim().to_string());
        } else if trimmed.starts_with("And") || trimmed.starts_with("But") {
            let step_text = trimmed
                .replace("And", "")
                .replace("But", "")
                .trim()
                .to_string();
            match current_step_type {
                "given" => given_steps.push(step_text),
                "when" => when_steps.push(step_text),
                "then" => then_steps.push(step_text),
                _ => {}
            }
        }
    }

    automation_api::AcceptanceCriteria {
        id: format!("req_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        title: title.clone(),
        feature: "Web Automation".to_string(),
        scenario,
        given_steps,
        when_steps,
        then_steps,
        tags: tags.to_vec(),
        priority: "normal".to_string(),
    }
}

async fn get_workflows(State(state): State<SharedState>) -> impl IntoResponse {
    let app_state = state.lock().await;
    let workflows = app_state.integration.get_all_workflows();

    let workflow_summaries: Vec<WorkflowSummary> = workflows
        .iter()
        .map(|w| WorkflowSummary {
            id: w.id.clone(),
            name: w.name.clone(),
            description: w.description.clone(),
            step_count: w.test_steps.len(),
            tags: w.tags.clone(),
        })
        .collect();

    let response = WorkflowListResponse {
        workflows: workflow_summaries,
    };

    (StatusCode::OK, Json(response))
}

async fn execute_workflow(
    State(state): State<SharedState>,
    axum::extract::Path(workflow_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let app_state = state.lock().await;

    if let Some(workflow) = app_state.integration.get_workflow(&workflow_id) {
        let workflow_clone = workflow.clone();
        drop(app_state); // Release the lock before async operation

        let mut app_state = state.lock().await;
        match app_state
            .integration
            .execute_automation_workflow(&workflow_clone)
            .await
        {
            Ok(report) => Json(report).into_response(),
            Err(e) => {
                let error_report = automation_browser::ExecutionReport::new(
                    "error",
                    &format!("Execution failed: {}", e),
                );
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error_report)).into_response()
            }
        }
    } else {
        let error_report = automation_browser::ExecutionReport::new("error", "Workflow not found");
        (StatusCode::NOT_FOUND, Json(error_report)).into_response()
    }
}

async fn get_script(
    State(state): State<SharedState>,
    axum::extract::Path(workflow_id): axum::extract::Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let app_state = state.lock().await;

    if let Some(workflow) = app_state.integration.get_workflow(&workflow_id) {
        let format = params
            .get("format")
            .unwrap_or(&"mcp_browser".to_string())
            .clone();
        let script = app_state
            .integration
            .generate_browser_script(workflow, &format);

        let content_type = match format.as_str() {
            "selenium" => "text/x-python",
            "playwright" => "application/javascript",
            _ => "application/javascript",
        };

        (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, content_type)],
            script,
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Workflow not found".to_string(),
        )
    }
}
