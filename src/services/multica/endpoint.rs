//! Resource-local descriptions of the upstream HTTP contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Paging {
    None,
    Single,
    Capped,
    Offset,
    OffsetUntilShort,
    Cursor,
    Comments,
    AgentTasks,
}

#[derive(Clone, Debug)]
pub(super) struct Endpoint {
    pub command: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub query: &'static str,
    pub fields: &'static str,
    pub paging: Paging,
    pub key: &'static str,
    pub page_size: usize,
    pub required: &'static str,
    pub workspace: bool,
}

impl Endpoint {
    // Positional entries keep the resource tables readable; this is a private contract table.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        command: &'static str,
        method: &'static str,
        path: &'static str,
        query: &'static str,
        fields: &'static str,
        paging: Paging,
        key: &'static str,
        page_size: usize,
        required: &'static str,
        workspace: bool,
    ) -> Self {
        Self {
            command,
            method,
            path,
            query,
            fields,
            paging,
            key,
            page_size,
            required,
            workspace,
        }
    }

    pub fn path_fields(&self) -> Vec<&str> {
        self.path
            .split('{')
            .skip(1)
            .filter_map(|s| s.split_once('}').map(|(key, _)| key))
            .collect()
    }
}
