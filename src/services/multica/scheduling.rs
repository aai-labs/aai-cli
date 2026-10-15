use super::endpoint::{Endpoint, Paging};

pub(super) fn endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint::new("autopilots list", "GET", "/autopilots", "status", "", Paging::Single, "autopilots", 100, "", true),
        Endpoint::new("autopilots get", "GET", "/autopilots/{autopilot_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots create", "POST", "/autopilots", "", "title description project_id assignee_type assignee_id execution_mode issue_title_template", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots update", "PATCH", "/autopilots/{autopilot_id}", "", "title description project_id assignee_type assignee_id execution_mode issue_title_template status", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots delete", "DELETE", "/autopilots/{autopilot_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots trigger", "POST", "/autopilots/{autopilot_id}/trigger", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots cron-preview", "GET", "/autopilots/cron-preview", "expr tz", "", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots runs list", "GET", "/autopilots/{autopilot_id}/runs", "", "", Paging::OffsetUntilShort, "runs", 100, "", true),
        Endpoint::new("autopilots runs get", "GET", "/autopilots/{autopilot_id}/runs/{run_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots triggers create", "POST", "/autopilots/{autopilot_id}/triggers", "", "cron_expression timezone", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots triggers update", "PATCH", "/autopilots/{autopilot_id}/triggers/{trigger_id}", "", "cron_expression timezone", Paging::None, "", 100, "", true),
        Endpoint::new("autopilots triggers delete", "DELETE", "/autopilots/{autopilot_id}/triggers/{trigger_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("issues wakeups list", "GET", "/issues/{issue_id}/wakeups", "", "", Paging::Single, "", 100, "", true),
        Endpoint::new("issues wakeups create", "POST", "/issues/{issue_id}/wakeups", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("issues wakeups set", "PUT", "/issues/{issue_id}/wakeups/{wakeup_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("issues wakeups edit-instruction", "PATCH", "/issues/{issue_id}/wakeups/{wakeup_id}/instruction", "", "instruction", Paging::None, "", 100, "", true),
        Endpoint::new("issues wakeups enable", "POST", "/issues/{issue_id}/wakeups/{wakeup_id}/enable", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("issues wakeups disable", "POST", "/issues/{issue_id}/wakeups/{wakeup_id}/disable", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("issues wakeups delete", "DELETE", "/issues/{issue_id}/wakeups/{wakeup_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("issues wakeups trigger", "POST", "/issues/{issue_id}/wakeups/{wakeup_id}/trigger", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("issues wakeups runs", "GET", "/issues/{issue_id}/wakeups/{wakeup_id}/runs", "", "", Paging::Capped, "", 100, "", true),
    ]
}
