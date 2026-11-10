use clap::{Args, Subcommand};

use crate::cli::GenericRequest;

#[derive(Debug, Args)]
pub struct GitlabCommand {
    #[command(subcommand)]
    pub resource: GitlabResource,
}

#[derive(Debug, Subcommand)]
pub enum GitlabResource {
    Projects(GitlabProjectsCommand),
    /// Merge requests.
    Mrs(GitlabMrsCommand),
    Issues(GitlabIssuesCommand),
    Branches(GitlabBranchesCommand),
    Commits(GitlabCommitsCommand),
    Source(GitlabSourceCommand),
    Pipelines(GitlabPipelinesCommand),
    Jobs(GitlabJobsCommand),
    /// Call an uncommon GitLab REST endpoint with profile authentication.
    /// Encode project paths in the endpoint: `projects/group%2Fapp/labels`.
    Request(GenericRequest),
}

/// Project selector shared by every project-scoped command.
#[derive(Debug, Args)]
pub struct GitlabProjectArg {
    /// Numeric project ID or full path such as `group/subgroup/app`. Defaults to
    /// profile.repo.
    #[arg(long)]
    pub project: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabLimit {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct GitlabProjectsCommand {
    #[command(subcommand)]
    pub action: GitlabProjectsAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabProjectsAction {
    List(GitlabProjectList),
    Get(GitlabProjectGet),
}

#[derive(Debug, Args)]
pub struct GitlabProjectList {
    /// Case-insensitive match on project name.
    #[arg(long)]
    pub search: Option<String>,
    /// Include every project visible to the token, not only those it is a member of.
    #[arg(long)]
    pub all: bool,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabProjectGet {
    /// Numeric project ID or full path. Defaults to profile.repo.
    pub project: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabMrsCommand {
    #[command(subcommand)]
    pub action: GitlabMrAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabMrAction {
    List(GitlabMrList),
    Get(GitlabIid),
    Create(GitlabMrCreate),
    Update(GitlabMrUpdate),
    Close(GitlabIid),
    Reopen(GitlabIid),
    /// Per-file diffs of a merge request.
    Diffs(GitlabMrDiffs),
    Commits(GitlabIidList),
    Approve(GitlabIid),
    Unapprove(GitlabIid),
    /// Approval state of a merge request.
    Approvals(GitlabIid),
    /// Comments on a merge request.
    Notes(GitlabNotesCommand),
    /// Comment threads, including comments on diff lines.
    Discussions(GitlabDiscussionsCommand),
}

#[derive(Debug, Args)]
pub struct GitlabIid {
    pub iid: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
}

#[derive(Debug, Args)]
pub struct GitlabIidList {
    pub iid: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabMrList {
    #[command(flatten)]
    pub project: GitlabProjectArg,
    /// `opened`, `closed`, `locked`, `merged`, or `all`. Omitted = provider default (all).
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub source_branch: Option<String>,
    #[arg(long)]
    pub target_branch: Option<String>,
    #[arg(long)]
    pub search: Option<String>,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabMrCreate {
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[arg(long)]
    pub source_branch: Option<String>,
    #[arg(long)]
    pub target_branch: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    /// JSON body, inline or from a path; use - to read stdin. Typed flags win.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabMrUpdate {
    pub iid: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub target_branch: Option<String>,
    /// JSON body, inline or from a path; use - to read stdin. Typed flags win.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabMrDiffs {
    pub iid: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[command(flatten)]
    pub limit: GitlabLimit,
    /// Write the JSON result to this path instead of stdout.
    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabNotesCommand {
    #[command(subcommand)]
    pub action: GitlabNoteAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabNoteAction {
    List(GitlabIidList),
    Get(GitlabNoteId),
    Create(GitlabNoteCreate),
    Update(GitlabNoteUpdate),
    Delete(GitlabNoteId),
}

#[derive(Debug, Args)]
pub struct GitlabNoteId {
    pub iid: u64,
    pub note_id: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
}

#[derive(Debug, Args)]
pub struct GitlabNoteCreate {
    pub iid: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[arg(long)]
    pub body: Option<String>,
    /// JSON body, inline or from a path; use - to read stdin. Typed flags win.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabNoteUpdate {
    pub iid: u64,
    pub note_id: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[arg(long)]
    pub body: Option<String>,
    /// JSON body, inline or from a path; use - to read stdin. Typed flags win.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabDiscussionsCommand {
    #[command(subcommand)]
    pub action: GitlabDiscussionAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabDiscussionAction {
    List(GitlabIidList),
    /// Start a thread. Pass `position` in --json to comment on a diff line.
    Create(GitlabNoteCreate),
}

#[derive(Debug, Args)]
pub struct GitlabIssuesCommand {
    #[command(subcommand)]
    pub action: GitlabIssueAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabIssueAction {
    List(GitlabIssueList),
    Get(GitlabIid),
    Create(GitlabIssueCreate),
    Update(GitlabIssueUpdate),
    /// Comments on an issue.
    Notes(GitlabIssueNotesCommand),
}

#[derive(Debug, Args)]
pub struct GitlabIssueList {
    #[command(flatten)]
    pub project: GitlabProjectArg,
    /// `opened`, `closed`, or `all`. Omitted = provider default (all).
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub search: Option<String>,
    #[arg(long)]
    pub labels: Option<String>,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabIssueCreate {
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    /// JSON body, inline or from a path; use - to read stdin. Typed flags win.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabIssueUpdate {
    pub iid: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    /// `close` or `reopen`.
    #[arg(long)]
    pub state_event: Option<String>,
    /// JSON body, inline or from a path; use - to read stdin. Typed flags win.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabIssueNotesCommand {
    #[command(subcommand)]
    pub action: GitlabIssueNoteAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabIssueNoteAction {
    List(GitlabIidList),
    Create(GitlabNoteCreate),
}

#[derive(Debug, Args)]
pub struct GitlabBranchesCommand {
    #[command(subcommand)]
    pub action: GitlabBranchAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabBranchAction {
    List(GitlabBranchList),
    Get(GitlabBranchName),
    Create(GitlabBranchCreate),
    Delete(GitlabBranchName),
}

#[derive(Debug, Args)]
pub struct GitlabBranchList {
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[arg(long)]
    pub search: Option<String>,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabBranchName {
    pub branch: String,
    #[command(flatten)]
    pub project: GitlabProjectArg,
}

#[derive(Debug, Args)]
pub struct GitlabBranchCreate {
    pub branch: String,
    /// Branch name or commit SHA to branch from.
    #[arg(long = "ref")]
    pub reference: String,
    #[command(flatten)]
    pub project: GitlabProjectArg,
}

#[derive(Debug, Args)]
pub struct GitlabCommitsCommand {
    #[command(subcommand)]
    pub action: GitlabCommitAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabCommitAction {
    List(GitlabCommitList),
    Get(GitlabCommitSha),
    Diff(GitlabCommitDiff),
}

#[derive(Debug, Args)]
pub struct GitlabCommitList {
    #[command(flatten)]
    pub project: GitlabProjectArg,
    /// Branch, tag, or SHA. Omitted = default branch.
    #[arg(long)]
    pub ref_name: Option<String>,
    /// Only commits touching this file path.
    #[arg(long)]
    pub path: Option<String>,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabCommitSha {
    pub sha: String,
    #[command(flatten)]
    pub project: GitlabProjectArg,
}

#[derive(Debug, Args)]
pub struct GitlabCommitDiff {
    pub sha: String,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabSourceCommand {
    #[command(subcommand)]
    pub action: GitlabSourceAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabSourceAction {
    /// Read one file at a branch, tag, or SHA.
    Get(GitlabSourceGet),
    /// List a directory.
    Tree(GitlabSourceTree),
}

#[derive(Debug, Args)]
pub struct GitlabSourceGet {
    /// Branch, tag, or SHA.
    #[arg(value_name = "REF")]
    pub reference: String,
    /// Path of the file in the repository.
    pub path: String,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    /// Write the file to this path instead of returning it. Needed for binary files.
    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct GitlabSourceTree {
    #[command(flatten)]
    pub project: GitlabProjectArg,
    /// Branch, tag, or SHA. Omitted = default branch.
    #[arg(long = "ref")]
    pub reference: Option<String>,
    /// Directory in the repository. Omitted = repository root.
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long)]
    pub recursive: bool,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabPipelinesCommand {
    #[command(subcommand)]
    pub action: GitlabPipelineAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabPipelineAction {
    List(GitlabPipelineList),
    Get(GitlabPipelineId),
    /// Jobs of a pipeline.
    Jobs(GitlabPipelineJobs),
    Retry(GitlabPipelineId),
    Cancel(GitlabPipelineId),
}

#[derive(Debug, Args)]
pub struct GitlabPipelineList {
    #[command(flatten)]
    pub project: GitlabProjectArg,
    /// Branch or tag.
    #[arg(long = "ref")]
    pub reference: Option<String>,
    /// For example `running`, `success`, or `failed`.
    #[arg(long)]
    pub status: Option<String>,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabPipelineId {
    pub pipeline_id: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
}

#[derive(Debug, Args)]
pub struct GitlabPipelineJobs {
    pub pipeline_id: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    #[command(flatten)]
    pub limit: GitlabLimit,
}

#[derive(Debug, Args)]
pub struct GitlabJobsCommand {
    #[command(subcommand)]
    pub action: GitlabJobAction,
}

#[derive(Debug, Subcommand)]
pub enum GitlabJobAction {
    Get(GitlabJobId),
    /// Read a job's log.
    Log(GitlabJobLog),
}

#[derive(Debug, Args)]
pub struct GitlabJobId {
    pub job_id: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
}

#[derive(Debug, Args)]
pub struct GitlabJobLog {
    pub job_id: u64,
    #[command(flatten)]
    pub project: GitlabProjectArg,
    /// Write the log to this path instead of returning it.
    #[arg(long)]
    pub output: Option<String>,
}
