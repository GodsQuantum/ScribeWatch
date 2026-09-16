use crate::{
    domain::{DashboardStats, JobStatus},
    state::AppState,
};
use axum::{Json, extract::State};

pub async fn stats(State(state): State<AppState>) -> Json<DashboardStats> {
    let workflows = state.workflows.read().await;
    let mut active_jobs = 0;
    let mut completed_jobs = 0;
    let mut failed_jobs = 0;
    for job in state.jobs.iter() {
        match job.status {
            JobStatus::Done => completed_jobs += 1,
            JobStatus::Error => failed_jobs += 1,
            _ if job.status.is_active() => active_jobs += 1,
            _ => {}
        }
    }
    Json(DashboardStats {
        workflows: workflows.len(),
        enabled_workflows: workflows.iter().filter(|workflow| workflow.enabled).count(),
        active_jobs,
        completed_jobs,
        failed_jobs,
    })
}
