use super::endpoint::{Endpoint, Paging};

pub(super) fn endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint::new("agents list", "GET", "/agents", "include_archived", "", Paging::Single, "", 100, "", true),
        Endpoint::new("agents get", "GET", "/agents/{agent_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents create", "POST", "/agents", "", "name description instructions avatar_url runtime_id visibility permission_mode max_concurrent_tasks model thinking_level service_tier", Paging::None, "", 100, "name", true),
        Endpoint::new("agents update", "PUT", "/agents/{agent_id}", "", "name description instructions avatar_url runtime_id visibility permission_mode max_concurrent_tasks model thinking_level service_tier", Paging::None, "", 100, "", true),
        Endpoint::new("agents archive", "POST", "/agents/{agent_id}/archive", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents restore", "POST", "/agents/{agent_id}/restore", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents cancel-tasks", "POST", "/agents/{agent_id}/cancel-tasks", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents tasks", "GET", "/agents/{agent_id}/tasks", "before include_usage", "", Paging::AgentTasks, "", 200, "", true),
        Endpoint::new("agents skills list", "GET", "/agents/{agent_id}/skills", "", "", Paging::Single, "", 100, "", true),
        Endpoint::new("agents skills add", "POST", "/agents/{agent_id}/skills/add", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents skills set", "PUT", "/agents/{agent_id}/skills", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents skills remove", "DELETE", "/agents/{agent_id}/skills/{skill_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents skills set-enabled", "PUT", "/agents/{agent_id}/skills/{skill_id}/enabled", "", "enabled", Paging::None, "", 100, "", true),
        Endpoint::new("agents mcp-servers list", "GET", "/agents/{agent_id}/mcp-servers", "", "", Paging::Single, "", 100, "", true),
        Endpoint::new("agents mcp-servers add", "POST", "/agents/{agent_id}/mcp-servers", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents mcp-servers remove", "DELETE", "/agents/{agent_id}/mcp-servers/{server_id}", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents mcp-servers set-enabled", "PUT", "/agents/{agent_id}/mcp-servers/{server_id}/enabled", "", "enabled", Paging::None, "", 100, "", true),
        Endpoint::new("agents runtime-skills set-enabled", "PUT", "/agents/{agent_id}/runtime-skills/enabled", "", "enabled", Paging::None, "", 100, "", true),
        Endpoint::new("agents env get", "GET", "/agents/{agent_id}/env", "", "", Paging::None, "", 100, "", true),
        Endpoint::new("agents env set", "PUT", "/agents/{agent_id}/env", "", "", Paging::None, "", 100, "", true),
    ]
}
