use super::endpoint::Endpoint;

pub(super) fn endpoints() -> Vec<Endpoint> {
    [
        super::issues::endpoints(),
        super::comments::endpoints(),
        super::inbox::endpoints(),
        super::squads::endpoints(),
        super::agents::endpoints(),
        super::projects::endpoints(),
        super::workspaces::endpoints(),
        super::catalogs::endpoints(),
        super::tasks::endpoints(),
        super::scheduling::endpoints(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
