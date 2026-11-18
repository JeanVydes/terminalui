//! Invented services and shared constants for the demo.

use terminalui::components::badge::BadgeVariant;

use crate::common::PAGE_SIZE;

pub(crate) const TOTAL_SERVICES: usize = 40;
const SERVICE_NAMES: [&str; 8] = [
    "web-api",
    "api-gateway",
    "worker",
    "billing",
    "auth",
    "search",
    "notifications",
    "scheduler",
];
const SERVICE_ENVS: [&str; 3] = ["production", "staging", "preview"];

pub(crate) struct Service {
    pub name: String,
    pub env: String,
    pub version: String,
    pub status: String,
}

pub(crate) fn services() -> Vec<Service> {
    (0..TOTAL_SERVICES)
        .map(|index| Service {
            name: format!(
                "{}-{:02}",
                SERVICE_NAMES[index % SERVICE_NAMES.len()],
                index
            ),
            env: SERVICE_ENVS[index % SERVICE_ENVS.len()].to_string(),
            version: format!("v2.{}.{}", index % 17, index % 5),
            status: match index % 13 {
                4 | 9 => "degraded".to_string(),
                6 | 11 => "down".to_string(),
                _ => "healthy".to_string(),
            },
        })
        .collect()
}

pub(crate) fn status_variant(status: &str) -> BadgeVariant {
    match status {
        "healthy" => BadgeVariant::Success,
        "degraded" => BadgeVariant::Warning,
        _ => BadgeVariant::Destructive,
    }
}

pub(crate) fn health_counts(services: &[Service]) -> (usize, usize, usize) {
    services.iter().fold(
        (0, 0, 0),
        |(healthy, degraded, down), service| match service.status.as_str() {
            "healthy" => (healthy + 1, degraded, down),
            "degraded" => (healthy, degraded + 1, down),
            _ => (healthy, degraded, down + 1),
        },
    )
}

pub(crate) fn total_pages(items: usize) -> usize {
    items.div_ceil(PAGE_SIZE).max(1)
}

pub(crate) fn selected_service<'a>(
    services: &'a [Service],
    view: &[usize],
    selected: usize,
) -> &'a Service {
    let index = view
        .get(selected.min(view.len().saturating_sub(1)))
        .copied()
        .unwrap_or(0);
    &services[index]
}
