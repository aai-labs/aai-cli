use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "aai-cli")]
#[command(about = "Agent-friendly CLI wrappers for common work APIs")]
#[command(
    long_about = "Agent-friendly CLI wrappers for common work APIs.\n\nEvery successful service response includes `_aai.pagination` with pagination status, continuation values, and instructions for retrieving more results. Bare provider arrays are wrapped under `results` so this metadata is always visible."
)]
pub struct Cli {
    #[arg(long, global = true, env = "AAI_PROFILE")]
    pub profile: Option<String>,
    #[arg(long, global = true, env = "AAI_CONFIG")]
    pub config: Option<String>,
    #[arg(long, global = true, env = "AAI_SECRETS_FILE")]
    pub secrets_file: Option<String>,
    #[arg(long, global = true, env = "AAI_SECRET_KEY_FILE")]
    pub key_file: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Jira(JiraCommand),
    Confluence(ConfluenceCommand),
    Bitbucket(BitbucketCommand),
    Github(GithubCommand),
    Email(EmailCommand),
    Calendar(CalendarCommand),
    /// Manage Pipedrive CRM records, history, activities, notes, and synced email.
    Pipedrive(PipedriveCommand),
    /// Manage Apollo CRM, search, outreach, workflow, and analytics APIs.
    Apollo(ApolloCommand),
    /// Manage HubSpot CRM, files, events, and conversations APIs.
    Hubspot(HubspotCommand),
    /// Read and write Google Sheets spreadsheets and cell data.
    Sheets(SheetsCommand),
    /// Read Google Drive files, folders, shared drives, and sharing; upload file content.
    Drive(DriveCommand),
    /// Read and write local spreadsheets: .xlsx/.xlsm and .csv/.tsv, plus read-only
    /// .xls/.xlsb/.ods. Needs no profile or credentials.
    Excel(ExcelCommand),
    /// Inspect and edit persistent profiles without exposing credentials.
    Config(ConfigCommand),
    /// Discover, validate, and install bundled Agent Skills.
    Skills(SkillsCommand),
    Secrets(SecretsCommand),
    /// Read Slack channels, files, bookmarks, links, and channel canvases.
    Slack(SlackCommand),
    /// Read OpenPanel projects, insights, profiles, and raw event exports.
    Openpanel(OpenpanelCommand),
    /// Read PostHog projects, queries, insights, persons, cohorts, dashboards, and annotations.
    Posthog(PosthogCommand),
    /// Authenticate to and call Microsoft Graph.
    Microsoft(MicrosoftCommand),
}

#[derive(Debug, Args)]
pub struct MicrosoftCommand {
    #[command(subcommand)]
    pub resource: MicrosoftResource,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftResource {
    /// Bootstrap or validate Microsoft authentication.
    Auth(MicrosoftAuthCommand),
    /// Upload, download, or delete OneDrive and SharePoint drive files.
    Files(MicrosoftFilesCommand),
    /// Read and manage Outlook messages, drafts, and sending.
    Mail(MicrosoftMailCommand),
    /// Read and manage Outlook calendar events.
    Calendar(MicrosoftCalendarCommand),
    /// Read and manage Outlook contacts.
    Contacts(MicrosoftContactsCommand),
    /// Read and manage SharePoint lists and list items.
    Sharepoint(MicrosoftSharepointCommand),
    /// Read Teams, channels, members, messages, and chats.
    Teams(MicrosoftTeamsCommand),
    /// Read and manage Microsoft To Do lists and tasks (delegated only).
    Todo(MicrosoftTodoCommand),
    /// Read Planner plans/buckets and manage Planner tasks.
    Planner(MicrosoftPlannerCommand),
    /// Call a Microsoft Graph endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct MicrosoftListArgs {
    /// Maximum number of resources to aggregate across Graph pages.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct MicrosoftUserArg {
    /// User ID or UPN. Defaults to profile.user_id.
    #[arg(long)]
    pub user_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftUserListArgs {
    #[command(flatten)]
    pub user: MicrosoftUserArg,
    #[command(flatten)]
    pub list: MicrosoftListArgs,
}

#[derive(Debug, Args)]
pub struct MicrosoftUserItemArg {
    pub id: String,
    #[command(flatten)]
    pub user: MicrosoftUserArg,
}

#[derive(Debug, Args)]
pub struct MicrosoftUserCreate {
    #[command(flatten)]
    pub user: MicrosoftUserArg,
    /// Provider JSON object, inline or from a path; use - to read stdin.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftUserUpdate {
    pub id: String,
    #[command(flatten)]
    pub user: MicrosoftUserArg,
    /// Provider JSON object, inline or from a path; use - to read stdin.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftMailCommand {
    #[command(subcommand)]
    pub resource: MicrosoftMailResource,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftMailResource {
    Messages(MicrosoftMessagesCommand),
    /// Send a message. JSON must contain the Graph sendMail request body.
    Send(MicrosoftUserCreate),
}

#[derive(Debug, Args)]
pub struct MicrosoftMessagesCommand {
    #[command(subcommand)]
    pub action: MicrosoftMessagesAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftMessagesAction {
    List(MicrosoftUserListArgs),
    Get(MicrosoftUserItemArg),
    /// Create a draft message.
    Create(MicrosoftUserCreate),
    Update(MicrosoftUserUpdate),
    Delete(MicrosoftUserItemArg),
}

#[derive(Debug, Args)]
pub struct MicrosoftCalendarCommand {
    #[command(subcommand)]
    pub resource: MicrosoftCalendarResource,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftCalendarResource {
    Events(MicrosoftEventsCommand),
}

#[derive(Debug, Args)]
pub struct MicrosoftEventsCommand {
    #[command(subcommand)]
    pub action: MicrosoftEventsAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftEventsAction {
    List(MicrosoftUserListArgs),
    Get(MicrosoftUserItemArg),
    Create(MicrosoftUserCreate),
    Update(MicrosoftUserUpdate),
    Delete(MicrosoftUserItemArg),
}

#[derive(Debug, Args)]
pub struct MicrosoftContactsCommand {
    #[command(subcommand)]
    pub action: MicrosoftContactsAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftContactsAction {
    List(MicrosoftUserListArgs),
    Get(MicrosoftUserItemArg),
    Create(MicrosoftUserCreate),
    Update(MicrosoftUserUpdate),
    Delete(MicrosoftUserItemArg),
}

#[derive(Debug, Args)]
pub struct MicrosoftSharepointCommand {
    #[command(subcommand)]
    pub resource: MicrosoftSharepointResource,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftSharepointResource {
    Lists(MicrosoftSharepointListsCommand),
    Items(MicrosoftSharepointItemsCommand),
}

#[derive(Debug, Args)]
pub struct MicrosoftSharepointListsCommand {
    #[command(subcommand)]
    pub action: MicrosoftSharepointListsAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftSharepointListsAction {
    List(MicrosoftSiteListArgs),
    Get(MicrosoftSiteListArg),
}

#[derive(Debug, Args)]
pub struct MicrosoftSharepointItemsCommand {
    #[command(subcommand)]
    pub action: MicrosoftSharepointItemsAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftSharepointItemsAction {
    List(MicrosoftListItemListArgs),
    Get(MicrosoftListItemArg),
    Create(MicrosoftListItemCreate),
    Update(MicrosoftListItemUpdate),
    Delete(MicrosoftListItemArg),
}

#[derive(Debug, Args)]
pub struct MicrosoftSiteListArgs {
    pub site_id: String,
    #[command(flatten)]
    pub list: MicrosoftListArgs,
}

#[derive(Debug, Args)]
pub struct MicrosoftSiteListArg {
    pub site_id: String,
    pub list_id: String,
}

#[derive(Debug, Args)]
pub struct MicrosoftListItemListArgs {
    pub site_id: String,
    pub list_id: String,
    #[command(flatten)]
    pub list: MicrosoftListArgs,
}

#[derive(Debug, Args)]
pub struct MicrosoftListItemArg {
    pub site_id: String,
    pub list_id: String,
    pub item_id: String,
}

#[derive(Debug, Args)]
pub struct MicrosoftListItemCreate {
    pub site_id: String,
    pub list_id: String,
    /// Graph listItem JSON. Usually `{ "fields": { ... } }`.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftListItemUpdate {
    pub site_id: String,
    pub list_id: String,
    pub item_id: String,
    /// Fields JSON object sent to the list item's `/fields` endpoint.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftTeamsCommand {
    #[command(subcommand)]
    pub action: MicrosoftTeamsAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftTeamsAction {
    Get(MicrosoftTeamArg),
    Channels(MicrosoftTeamListArgs),
    Channel(MicrosoftChannelArg),
    Members(MicrosoftTeamListArgs),
    Messages(MicrosoftChannelListArgs),
    Chats(MicrosoftUserListArgs),
}

#[derive(Debug, Args)]
pub struct MicrosoftTeamArg {
    pub team_id: String,
}

#[derive(Debug, Args)]
pub struct MicrosoftTeamListArgs {
    pub team_id: String,
    #[command(flatten)]
    pub list: MicrosoftListArgs,
}

#[derive(Debug, Args)]
pub struct MicrosoftChannelArg {
    pub team_id: String,
    pub channel_id: String,
}

#[derive(Debug, Args)]
pub struct MicrosoftChannelListArgs {
    pub team_id: String,
    pub channel_id: String,
    #[command(flatten)]
    pub list: MicrosoftListArgs,
}

#[derive(Debug, Args)]
pub struct MicrosoftTodoCommand {
    #[command(subcommand)]
    pub resource: MicrosoftTodoResource,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftTodoResource {
    Lists(MicrosoftTodoListsCommand),
    Tasks(MicrosoftTodoTasksCommand),
}

#[derive(Debug, Args)]
pub struct MicrosoftTodoListsCommand {
    #[command(subcommand)]
    pub action: MicrosoftTodoListsAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftTodoListsAction {
    List(MicrosoftUserListArgs),
    Get(MicrosoftUserItemArg),
    Create(MicrosoftUserCreate),
    Update(MicrosoftUserUpdate),
    Delete(MicrosoftUserItemArg),
}

#[derive(Debug, Args)]
pub struct MicrosoftTodoTasksCommand {
    #[command(subcommand)]
    pub action: MicrosoftTodoTasksAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftTodoTasksAction {
    List(MicrosoftTodoTaskListArgs),
    Get(MicrosoftTodoTaskArg),
    Create(MicrosoftTodoTaskCreate),
    Update(MicrosoftTodoTaskUpdate),
    Delete(MicrosoftTodoTaskArg),
}

#[derive(Debug, Args)]
pub struct MicrosoftTodoTaskListArgs {
    pub list_id: String,
    #[command(flatten)]
    pub user: MicrosoftUserArg,
    #[command(flatten)]
    pub list: MicrosoftListArgs,
}

#[derive(Debug, Args)]
pub struct MicrosoftTodoTaskArg {
    pub list_id: String,
    pub task_id: String,
    #[command(flatten)]
    pub user: MicrosoftUserArg,
}

#[derive(Debug, Args)]
pub struct MicrosoftTodoTaskCreate {
    pub list_id: String,
    #[command(flatten)]
    pub user: MicrosoftUserArg,
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftTodoTaskUpdate {
    pub list_id: String,
    pub task_id: String,
    #[command(flatten)]
    pub user: MicrosoftUserArg,
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftPlannerCommand {
    #[command(subcommand)]
    pub resource: MicrosoftPlannerResource,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftPlannerResource {
    Plans(MicrosoftPlannerPlansCommand),
    Buckets(MicrosoftPlannerBucketsCommand),
    Tasks(MicrosoftPlannerTasksCommand),
}

#[derive(Debug, Args)]
pub struct MicrosoftPlannerPlansCommand {
    #[command(subcommand)]
    pub action: MicrosoftPlannerGetAction,
}

#[derive(Debug, Args)]
pub struct MicrosoftPlannerBucketsCommand {
    #[command(subcommand)]
    pub action: MicrosoftPlannerGetAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftPlannerGetAction {
    Get(IdArg),
}

#[derive(Debug, Args)]
pub struct MicrosoftPlannerTasksCommand {
    #[command(subcommand)]
    pub action: MicrosoftPlannerTasksAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftPlannerTasksAction {
    Get(IdArg),
    Create(MicrosoftJsonCreate),
    Update(MicrosoftPlannerTaskUpdate),
    Delete(MicrosoftPlannerTaskDelete),
}

#[derive(Debug, Args)]
pub struct MicrosoftJsonCreate {
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftPlannerTaskUpdate {
    pub id: String,
    /// Last observed Planner task ETag.
    #[arg(long)]
    pub etag: String,
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftPlannerTaskDelete {
    pub id: String,
    /// Last observed Planner task ETag.
    #[arg(long)]
    pub etag: String,
}

#[derive(Debug, Args)]
pub struct MicrosoftFilesCommand {
    #[command(subcommand)]
    pub action: MicrosoftFilesAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftFilesAction {
    Upload(MicrosoftFileUpload),
    Download(MicrosoftFileDownload),
    Delete(MicrosoftFileTarget),
}

#[derive(Debug, Args)]
pub struct MicrosoftFileTarget {
    /// Path relative to the drive root.
    pub path: String,
    /// SharePoint or OneDrive drive ID. Omit to use the configured user's OneDrive.
    #[arg(long, conflicts_with = "user_id")]
    pub drive_id: Option<String>,
    /// User ID whose OneDrive should be used. Defaults to profile.user_id.
    #[arg(long, conflicts_with = "drive_id")]
    pub user_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct MicrosoftFileUpload {
    /// Local file containing the bytes to upload.
    pub file: String,
    #[command(flatten)]
    pub target: MicrosoftFileTarget,
    /// MIME type sent to Microsoft Graph.
    #[arg(long, default_value = "application/octet-stream")]
    pub mime_type: String,
}

#[derive(Debug, Args)]
pub struct MicrosoftFileDownload {
    #[command(flatten)]
    pub target: MicrosoftFileTarget,
    /// Local output path. File content is never written to stdout.
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct MicrosoftAuthCommand {
    #[command(subcommand)]
    pub action: MicrosoftAuthAction,
}

#[derive(Debug, Subcommand)]
pub enum MicrosoftAuthAction {
    /// Sign in once with device code and save the refresh token encrypted.
    Login,
    /// Prove saved credentials can acquire a token without interaction.
    Status,
}

#[derive(Debug, Args)]
pub struct SkillsCommand {
    #[command(subcommand)]
    pub action: SkillsAction,
}

#[derive(Debug, Subcommand)]
pub enum SkillsAction {
    /// List bundled Agent Skills and command coverage.
    Discover,
    /// Install one bundled skill, or every bundled skill with --all.
    Install(SkillsInstall),
    /// Validate bundled skill packages.
    Validate(SkillsValidate),
}

#[derive(Debug, Args)]
pub struct SkillsInstall {
    /// Bundled skill package name to install.
    pub skill_name: Option<String>,
    /// Install every bundled skill.
    #[arg(long, conflicts_with = "skill_name")]
    pub all: bool,
    /// Target Agent Skills directory. Defaults to ~/.agents/skills.
    #[arg(long)]
    pub target_dir: Option<PathBuf>,
    /// Overwrite existing installed skill directories.
    #[arg(long)]
    pub force: bool,
    /// Report planned writes without changing the filesystem.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct SkillsValidate {
    /// Bundled skill package name to validate. Omit to validate all.
    pub skill_name: Option<String>,
}

#[derive(Debug, Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub resource: ConfigResource,
}

#[derive(Debug, Subcommand)]
pub enum ConfigResource {
    /// Inspect, create, update, validate, or remove profiles.
    Profiles(ConfigProfilesCommand),
    /// Inspect or change the configured default profile.
    DefaultProfile(ConfigDefaultProfileCommand),
}

#[derive(Debug, Args)]
pub struct ConfigProfilesCommand {
    #[command(subcommand)]
    pub action: ConfigProfilesAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfigProfilesAction {
    List,
    Get(ConfigProfileName),
    Set(ConfigProfileSet),
    Remove(ConfigProfileName),
    Validate(ConfigProfileName),
}

#[derive(Debug, Args)]
pub struct ConfigProfileName {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct ConfigProfileSet {
    pub name: String,
    /// JSON object, inline or from a path; use - to read stdin.
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub auth_type: Option<String>,
    #[arg(long)]
    pub base_url: Option<String>,
    #[arg(long)]
    pub api_token_secret: Option<String>,
    #[arg(long)]
    pub token_secret: Option<String>,
    #[arg(long)]
    pub password_secret: Option<String>,
}

#[derive(Debug, Args)]
pub struct ConfigDefaultProfileCommand {
    #[command(subcommand)]
    pub action: ConfigDefaultProfileAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfigDefaultProfileAction {
    Get,
    Set(ConfigProfileName),
}

#[derive(Debug, Args)]
pub struct SecretsCommand {
    #[command(subcommand)]
    pub action: SecretsAction,
}

#[derive(Debug, Subcommand)]
pub enum SecretsAction {
    Set(SecretSet),
    List,
    Remove(SecretKeyArg),
}

#[derive(Debug, Args)]
pub struct SecretSet {
    pub key: String,
    #[arg(long)]
    pub value: Option<String>,
}

#[derive(Debug, Args)]
pub struct SecretKeyArg {
    pub key: String,
}

#[derive(Debug, Args)]
pub struct GenericRequest {
    #[arg(value_enum)]
    pub method: GenericHttpMethod,
    /// Relative provider endpoint path. Absolute URLs are rejected.
    pub path: String,
    /// Query parameter as key=value. Repeat for multiple parameters.
    #[arg(long = "query")]
    pub query: Vec<String>,
    /// JSON body, inline or from a path; use - to read stdin.
    #[arg(long)]
    pub json: Option<String>,
    /// Required for POST, PUT, PATCH, and DELETE.
    #[arg(long)]
    pub allow_write: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GenericHttpMethod {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Args)]
pub struct JiraCommand {
    #[command(subcommand)]
    pub resource: JiraResource,
}

#[derive(Debug, Subcommand)]
pub enum JiraResource {
    Issues(JiraIssuesCommand),
    /// Manage Jira Product Discovery ideas and discover project-specific idea fields.
    Ideas(JiraIdeasCommand),
    Projects(JiraProjectsCommand),
    Sprints(JiraSprintsCommand),
    Boards(JiraBoardsCommand),
    Users(JiraUsersCommand),
    /// Call an uncommon Jira REST endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct JiraIssuesCommand {
    #[command(subcommand)]
    pub action: JiraIssuesAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraIssuesAction {
    List(JiraIssueList),
    Get(IdArg),
    Create(JiraIssueCreate),
    Update(JiraIssueUpdate),
    Delete(IdArg),
    Comments(JiraIssueCommentsCommand),
    Attachments(JiraIssueAttachmentsCommand),
}

#[derive(Debug, Args)]
pub struct JiraIssueCommentsCommand {
    #[command(subcommand)]
    pub action: JiraIssueCommentsAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraIssueCommentsAction {
    List(JiraIssueCommentsList),
    Get(JiraIssueCommentsGet),
    Create(JiraIssueCommentsCreate),
}

#[derive(Debug, Args)]
pub struct JiraIssueCommentsList {
    pub issue: String,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct JiraIssueCommentsGet {
    pub issue: String,
    pub comment: String,
}

#[derive(Debug, Args)]
pub struct JiraIssueCommentsCreate {
    pub issue: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
}

#[derive(Debug, Args)]
pub struct JiraIssueAttachmentsCommand {
    #[command(subcommand)]
    pub action: JiraIssueAttachmentsAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraIssueAttachmentsAction {
    List(JiraIssueAttachmentsList),
    Download(JiraAttachmentDownload),
    Upload(JiraAttachmentUpload),
}

#[derive(Debug, Args)]
pub struct JiraIssueAttachmentsList {
    pub issue: String,
}

#[derive(Debug, Args)]
pub struct JiraAttachmentDownload {
    pub attachment_id: String,
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct JiraAttachmentUpload {
    pub issue: String,
    #[arg(long)]
    pub file: String,
}

#[derive(Debug, Args)]
pub struct JiraIssueList {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub assignee: Option<String>,
    #[arg(long = "type")]
    pub issue_type: Option<String>,
    #[arg(long)]
    pub sprint: Option<String>,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long = "updated-since")]
    pub updated_since: Option<String>,
    #[arg(long)]
    pub fields: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct JiraIssueCreate {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub issue_type: Option<String>,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct JiraIssueUpdate {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct JiraIdeasCommand {
    #[command(subcommand)]
    pub action: JiraIdeasAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraIdeasAction {
    /// Search ideas across Jira Product Discovery projects.
    List(JiraIdeasList),
    Get(IdArg),
    Create(JiraIdeasCreate),
    Update(JiraIdeasUpdate),
    /// Discover the fields available on a Product Discovery project's idea type.
    Fields(JiraIdeasFields),
    /// Parse a Product Discovery view or idea URL into its project, view, and issue parts.
    ParseUrl(JiraIdeasParseUrl),
    /// List or perform workflow transitions for an idea.
    Transitions(JiraIdeaTransitionsCommand),
}

#[derive(Debug, Args)]
pub struct JiraIdeasList {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub assignee: Option<String>,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long = "updated-since")]
    pub updated_since: Option<String>,
    #[arg(long)]
    pub fields: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct JiraIdeasCreate {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub project: Option<String>,
    /// Issue type name for the idea. Defaults to Idea.
    #[arg(long = "type")]
    pub issue_type: Option<String>,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct JiraIdeasUpdate {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct JiraIdeasFields {
    /// Product Discovery project key or id.
    pub project: String,
    /// Issue type name to inspect when the project has more than one.
    #[arg(long = "type")]
    pub issue_type: Option<String>,
    #[arg(long, default_value_t = 200)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct JiraIdeasParseUrl {
    pub url: String,
}

#[derive(Debug, Args)]
pub struct JiraIdeaTransitionsCommand {
    #[command(subcommand)]
    pub action: JiraIdeaTransitionsAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraIdeaTransitionsAction {
    /// List workflow transitions currently available for an idea.
    List(IdArg),
    /// Perform a workflow transition by id or exact name (case-insensitive).
    Perform(JiraIdeaTransitionPerform),
}

#[derive(Debug, Args)]
pub struct JiraIdeaTransitionPerform {
    pub id: String,
    /// Available transition id or name, for example 31 or Research.
    #[arg(long)]
    pub transition: String,
    /// Optional Jira transition body; --transition overrides its transition.id.
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct JiraProjectsCommand {
    #[command(subcommand)]
    pub action: JiraProjectsAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraProjectsAction {
    List(JiraProjectsList),
    Get(IdArg),
}

#[derive(Debug, Args)]
pub struct JiraProjectsList {
    /// Filter by project type key: product_discovery, software, business, or service_desk.
    #[arg(long = "type")]
    pub project_type: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct JiraSprintsCommand {
    #[command(subcommand)]
    pub action: JiraSprintsAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraSprintsAction {
    List(JiraSprintsList),
    Get(JiraSprintsGet),
    Create(JiraSprintsCreate),
    Issues(JiraSprintsIssuesCommand),
}

#[derive(Debug, Args)]
pub struct JiraSprintsIssuesCommand {
    #[command(subcommand)]
    pub action: JiraSprintsIssuesAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraSprintsIssuesAction {
    Add(JiraSprintsIssuesAdd),
}

#[derive(Debug, Args)]
pub struct JiraSprintsIssuesAdd {
    pub sprint: u64,
    #[arg(long)]
    pub issues: String,
}

#[derive(Debug, Args)]
pub struct JiraSprintsList {
    #[arg(long)]
    pub board: u64,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct JiraSprintsGet {
    pub id: u64,
}

#[derive(Debug, Args)]
pub struct JiraSprintsCreate {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub board: Option<u64>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub goal: Option<String>,
    #[arg(long = "start-date")]
    pub start_date: Option<String>,
    #[arg(long = "end-date")]
    pub end_date: Option<String>,
}

#[derive(Debug, Args)]
pub struct JiraBoardsCommand {
    #[command(subcommand)]
    pub action: JiraBoardsAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraBoardsAction {
    List(JiraBoardsList),
    Get(JiraBoardsGet),
}

#[derive(Debug, Args)]
pub struct JiraBoardsList {
    #[arg(long = "type")]
    pub board_type: Option<String>,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct JiraBoardsGet {
    pub id: u64,
}

#[derive(Debug, Args)]
pub struct JiraUsersCommand {
    #[command(subcommand)]
    pub action: JiraUsersAction,
}

#[derive(Debug, Subcommand)]
pub enum JiraUsersAction {
    Get(JiraUsersGet),
}

#[derive(Debug, Args)]
pub struct JiraUsersGet {
    pub account_id: String,
}

#[derive(Debug, Args)]
pub struct ConfluenceCommand {
    #[command(subcommand)]
    pub resource: ConfluenceResource,
}

#[derive(Debug, Subcommand)]
pub enum ConfluenceResource {
    Spaces(ConfluenceSpacesCommand),
    Pages(ConfluencePagesCommand),
    /// Call an uncommon Confluence REST endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct ConfluenceSpacesCommand {
    #[command(subcommand)]
    pub action: ConfluenceSpacesAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfluenceSpacesAction {
    List(ConfluenceSpacesList),
    Get(IdArg),
}

#[derive(Debug, Args)]
pub struct ConfluenceSpacesList {
    #[arg(long = "type")]
    pub space_type: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub key: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct ConfluencePagesCommand {
    #[command(subcommand)]
    pub action: ConfluencePagesAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfluencePagesAction {
    List(ConfluencePagesList),
    Get(IdArg),
    Create(ConfluencePageCreate),
    Update(ConfluencePageUpdate),
    Move(ConfluencePageMove),
    Delete(IdArg),
    Comments(ConfluencePageCommentsCommand),
    Attachments(ConfluencePageAttachmentsCommand),
}

#[derive(Debug, Args)]
pub struct ConfluencePageCommentsCommand {
    #[command(subcommand)]
    pub action: ConfluencePageCommentsAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfluencePageCommentsAction {
    List(ConfluencePageCommentsList),
    Create(ConfluencePageCommentsCreate),
}

#[derive(Debug, Args)]
pub struct ConfluencePageCommentsList {
    pub page_id: String,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct ConfluencePageCommentsCreate {
    pub page_id: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long = "reply-to")]
    pub reply_to: Option<String>,
}

#[derive(Debug, Args)]
pub struct ConfluencePageAttachmentsCommand {
    #[command(subcommand)]
    pub action: ConfluencePageAttachmentsAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfluencePageAttachmentsAction {
    List(ConfluencePageAttachmentsList),
    Download(ConfluencePageAttachmentsDownload),
    Upload(ConfluencePageAttachmentsUpload),
}

#[derive(Debug, Args)]
pub struct ConfluencePageAttachmentsList {
    pub page_id: String,
    #[arg(long, default_value_t = 25)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct ConfluencePageAttachmentsDownload {
    pub page_id: String,
    pub attachment_id: String,
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct ConfluencePageAttachmentsUpload {
    pub page_id: String,
    #[arg(long)]
    pub file: String,
    #[arg(long)]
    pub comment: Option<String>,
}

#[derive(Debug, Args)]
pub struct ConfluencePagesList {
    #[arg(long)]
    pub space: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub parent: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct ConfluencePageCreate {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub space_id: Option<String>,
    #[arg(long)]
    pub space_key: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ConfluencePageUpdate {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub version: Option<u64>,
}

#[derive(Debug, Args)]
pub struct ConfluencePageMove {
    pub id: String,
    #[arg(long)]
    pub target_id: String,
    #[arg(long, value_enum, default_value_t = ConfluenceMovePosition::Append)]
    pub position: ConfluenceMovePosition,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ConfluenceMovePosition {
    Before,
    After,
    Append,
}

impl ConfluenceMovePosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
            Self::Append => "append",
        }
    }
}

#[derive(Debug, Args)]
pub struct BitbucketCommand {
    #[command(subcommand)]
    pub resource: BitbucketResource,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketResource {
    Repos(BitbucketReposCommand),
    Prs(BitbucketPrsCommand),
    Branches(BitbucketBranchesCommand),
    Commits(BitbucketCommitsCommand),
    Source(BitbucketSourceCommand),
    Pipelines(BitbucketPipelinesCommand),
    /// Call an uncommon Bitbucket REST endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct BitbucketReposCommand {
    #[command(subcommand)]
    pub action: ListGetAction,
}

#[derive(Debug, Args)]
pub struct BitbucketPrsCommand {
    #[command(subcommand)]
    pub action: BitbucketPullRequestAction,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketPullRequestAction {
    List(PrListArg),
    Get(NumberArg),
    Create(PullRequestCreate),
    Delete(NumberArg),
    Close(NumberArg),
    Decline(NumberArg),
    Diff(BitbucketPrDiff),
    Diffstat(BitbucketPrDiffstat),
    Commits(BitbucketPrCommits),
    Activity(BitbucketPrActivity),
    Comments(BitbucketPrCommentsCommand),
}

#[derive(Debug, Args)]
pub struct BitbucketPrCommentsCommand {
    #[command(subcommand)]
    pub action: BitbucketPrCommentAction,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketPrCommentAction {
    List(BitbucketPrCommentList),
    Get(BitbucketPrCommentGet),
    Create(BitbucketPrCommentWrite),
    Update(BitbucketPrCommentWrite),
    Delete(BitbucketPrCommentGet),
}

#[derive(Debug, Args)]
pub struct BitbucketPrCommentList {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub inline_only: bool,
}

#[derive(Debug, Args)]
pub struct BitbucketPrCommentGet {
    pub pr: u64,
    pub comment: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketPrCommentWrite {
    pub pr: u64,
    #[arg(long)]
    pub comment: Option<u64>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub inline_path: Option<String>,
    #[arg(long)]
    pub inline_from: Option<u64>,
    #[arg(long)]
    pub inline_to: Option<u64>,
    #[arg(long)]
    pub parent_id: Option<u64>,
}

#[derive(Debug, Args)]
pub struct BitbucketPrDiff {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketPrDiffstat {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct BitbucketPrCommits {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct BitbucketPrActivity {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct BitbucketBranchesCommand {
    #[command(subcommand)]
    pub action: BitbucketBranchesAction,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketBranchesAction {
    List(BitbucketBranchList),
    Get(BitbucketBranchGet),
}

#[derive(Debug, Args)]
pub struct BitbucketBranchList {
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long, conflicts_with_all = ["name_prefix", "query"])]
    pub name_contains: Option<String>,
    #[arg(long, conflicts_with_all = ["name_contains", "query"])]
    pub name_prefix: Option<String>,
    #[arg(
        long,
        hide = true,
        help = "Advanced escape hatch: raw Bitbucket BBQL expression for the ?q= filter (e.g. 'name ~ \"^feature/\"'). Prefer --name-contains or --name-prefix."
    )]
    pub query: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketBranchGet {
    pub name: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketCommitsCommand {
    #[command(subcommand)]
    pub action: BitbucketCommitsAction,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketCommitsAction {
    List(BitbucketCommitList),
    Get(BitbucketCommitGet),
}

#[derive(Debug, Args)]
pub struct BitbucketCommitList {
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub include: Option<String>,
    #[arg(long)]
    pub exclude: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketCommitGet {
    pub sha: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketSourceCommand {
    #[command(subcommand)]
    pub action: BitbucketSourceAction,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketSourceAction {
    Get(BitbucketSourceGet),
    History(BitbucketSourceHistory),
}

#[derive(Debug, Args)]
pub struct BitbucketSourceGet {
    pub commit: String,
    pub path: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, conflicts_with = "meta")]
    pub output: Option<String>,
    #[arg(long, conflicts_with = "output")]
    pub meta: bool,
}

#[derive(Debug, Args)]
pub struct BitbucketSourceHistory {
    pub commit: String,
    pub path: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct BitbucketPipelinesCommand {
    #[command(subcommand)]
    pub action: BitbucketPipelinesAction,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketPipelinesAction {
    List(BitbucketPipelineList),
    Get(BitbucketPipelineGet),
    Steps(BitbucketPipelineStepsCommand),
}

#[derive(Debug, Args)]
pub struct BitbucketPipelineList {
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub sort: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketPipelineGet {
    pub pipeline: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketPipelineStepsCommand {
    #[command(subcommand)]
    pub action: BitbucketPipelineStepsAction,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketPipelineStepsAction {
    List(BitbucketPipelineStepList),
    Get(BitbucketPipelineStepGet),
    Logs(BitbucketPipelineStepLogsCommand),
}

#[derive(Debug, Args)]
pub struct BitbucketPipelineStepList {
    pub pipeline: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketPipelineStepGet {
    pub pipeline: String,
    pub step: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct BitbucketPipelineStepLogsCommand {
    #[command(subcommand)]
    pub action: BitbucketPipelineStepLogsAction,
}

#[derive(Debug, Subcommand)]
pub enum BitbucketPipelineStepLogsAction {
    Download(BitbucketPipelineStepLogDownload),
}

#[derive(Debug, Args)]
pub struct BitbucketPipelineStepLogDownload {
    pub pipeline: String,
    pub step: String,
    #[arg(long)]
    pub log: Option<String>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct GithubCommand {
    #[command(subcommand)]
    pub resource: GithubResource,
}

#[derive(Debug, Subcommand)]
pub enum GithubResource {
    Repos(GithubReposCommand),
    Issues(GithubIssuesCommand),
    Prs(GithubPrsCommand),
    Actions(GithubActionsCommand),
    Branches(GithubBranchesCommand),
    Source(GithubSourceCommand),
    /// Call an uncommon GitHub REST endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct GithubReposCommand {
    #[command(subcommand)]
    pub action: GithubReposAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubReposAction {
    List(LimitArg),
    Get(RepoArg),
}

#[derive(Debug, Args)]
pub struct GithubIssuesCommand {
    #[command(subcommand)]
    pub action: GithubIssuesAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubIssuesAction {
    List(RepoLimitArg),
    Get(NumberArg),
    Create(GithubIssueCreate),
    Update(GithubIssueUpdate),
    Delete(NumberArg),
}

#[derive(Debug, Args)]
pub struct GithubIssueCreate {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubIssueUpdate {
    pub number: u64,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubPrsCommand {
    #[command(subcommand)]
    pub action: GithubPullRequestAction,
}

#[derive(Debug, Args)]
pub struct GithubActionsCommand {
    #[command(subcommand)]
    pub resource: GithubActionsResource,
}

#[derive(Debug, Subcommand)]
pub enum GithubActionsResource {
    Runs(GithubActionsRunsCommand),
    Jobs(GithubActionsJobsCommand),
}

#[derive(Debug, Args)]
pub struct GithubActionsRunsCommand {
    #[command(subcommand)]
    pub action: GithubActionsRunsAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubActionsRunsAction {
    List(GithubActionsRunList),
    Get(GithubActionsRunGet),
    Logs(GithubActionsRunLogsCommand),
}

#[derive(Debug, Args)]
pub struct GithubActionsRunList {
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub event: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubActionsRunGet {
    pub run: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubActionsRunLogsCommand {
    #[command(subcommand)]
    pub action: GithubActionsRunLogsAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubActionsRunLogsAction {
    Download(GithubActionsRunLogDownload),
}

#[derive(Debug, Args)]
pub struct GithubActionsRunLogDownload {
    pub run: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct GithubActionsJobsCommand {
    #[command(subcommand)]
    pub action: GithubActionsJobsAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubActionsJobsAction {
    List(GithubActionsJobList),
    Get(GithubActionsJobGet),
    Logs(GithubActionsJobLogsCommand),
}

#[derive(Debug, Args)]
pub struct GithubActionsJobList {
    pub run: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub all_attempts: bool,
}

#[derive(Debug, Args)]
pub struct GithubActionsJobGet {
    pub job: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubActionsJobLogsCommand {
    #[command(subcommand)]
    pub action: GithubActionsJobLogsAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubActionsJobLogsAction {
    Download(GithubActionsJobLogDownload),
}

#[derive(Debug, Args)]
pub struct GithubActionsJobLogDownload {
    pub job: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct EmailCommand {
    #[command(subcommand)]
    pub resource: EmailResource,
}

#[derive(Debug, Subcommand)]
pub enum EmailResource {
    Messages(EmailMessagesCommand),
    /// Call an uncommon REST email endpoint. SMTP/IMAP profiles are rejected.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct EmailMessagesCommand {
    #[command(subcommand)]
    pub action: EmailMessagesAction,
}

#[derive(Debug, Subcommand)]
pub enum EmailMessagesAction {
    List(EmailMessageList),
    Get(IdArg),
    Send(EmailSend),
    Delete(IdArg),
}

#[derive(Debug, Args)]
pub struct EmailMessageList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub received_after: Option<String>,
    #[arg(long)]
    pub received_before: Option<String>,
}

#[derive(Debug, Args)]
pub struct EmailSend {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long)]
    pub subject: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
}

#[derive(Debug, Args)]
pub struct CalendarCommand {
    #[command(subcommand)]
    pub resource: CalendarResource,
}

#[derive(Debug, Subcommand)]
pub enum CalendarResource {
    Events(CalendarEventsCommand),
    /// Call an uncommon REST calendar endpoint. CalDAV profiles are rejected.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct CalendarEventsCommand {
    #[command(subcommand)]
    pub action: CalendarEventsAction,
}

#[derive(Debug, Subcommand)]
pub enum CalendarEventsAction {
    List(CalendarEventList),
    Get(IdArg),
    Create(CalendarEventCreate),
    Update(CalendarEventUpdate),
    Delete(CalendarEventDelete),
}

#[derive(Debug, Args)]
pub struct CalendarEventDelete {
    pub id: String,
    #[arg(long)]
    pub calendar_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct CalendarEventList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub calendar_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct CalendarEventCreate {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub calendar_id: Option<String>,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub start: Option<String>,
    #[arg(long)]
    pub end: Option<String>,
}

#[derive(Debug, Args)]
pub struct CalendarEventUpdate {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub calendar_id: Option<String>,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub start: Option<String>,
    #[arg(long)]
    pub end: Option<String>,
}

#[derive(Debug, Args)]
#[command(
    about = "Manage Pipedrive CRM records and communication history",
    long_about = "Manage Pipedrive leads, persons, organizations, deals, labels, activities, notes, and synced email history.\n\nUse `deals view`, `persons view`, or `organizations view` for a combined record, activities, and notes response. Add `--include-mail` to include associated email history.",
    after_help = "Examples:\n  aai-cli pipedrive deals view 123 --include-mail\n  aai-cli pipedrive persons activities 456 --limit 25\n  aai-cli pipedrive notes list --deal-id 123\n  aai-cli pipedrive mailbox messages get 789 --include-body"
)]
pub struct PipedriveCommand {
    #[command(subcommand)]
    pub resource: PipedriveResource,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveResource {
    /// Manage leads in the Leads Inbox.
    Leads(PipedriveLeadsCommand),
    /// Manage persons (contacts) and inspect their full CRM history.
    Persons(PipedrivePersonsCommand),
    /// Manage organizations and inspect their full CRM history.
    Organizations(PipedriveOrganizationsCommand),
    /// Manage deals and inspect their full CRM history.
    Deals(PipedriveDealsCommand),
    /// List and manage CRM labels.
    Labels(PipedriveLabelsCommand),
    /// List or get activities across CRM records.
    Activities(PipedriveActivitiesCommand),
    /// List or get notes across CRM records.
    Notes(PipedriveNotesCommand),
    /// Inspect synced email messages and threads.
    Mailbox(PipedriveMailboxCommand),
    /// Call an uncommon Pipedrive REST endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct PipedriveLeadsCommand {
    #[command(subcommand)]
    pub action: PipedriveLeadsAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveLeadsAction {
    List(PipedriveLeadList),
    Search(PipedriveLeadSearch),
    Get(PipedriveIdArg),
    Create(PipedriveLeadWrite),
    Update(PipedriveLeadUpdate),
    Delete(PipedriveIdArg),
    Convert(PipedriveLeadConvert),
}

#[derive(Debug, Args)]
pub struct PipedriveLeadList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub owner_id: Option<String>,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub organization_id: Option<String>,
    #[arg(long)]
    pub filter_id: Option<String>,
    #[arg(long)]
    pub updated_since: Option<String>,
    #[arg(long)]
    pub sort: Option<String>,
    #[arg(long)]
    pub archived: bool,
}

#[derive(Debug, Args)]
pub struct PipedriveLeadSearch {
    #[arg(long)]
    pub term: String,
    #[arg(long)]
    pub fields: Option<String>,
    #[arg(long)]
    pub exact_match: bool,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub organization_id: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct PipedriveLeadWrite {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub organization_id: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveLeadUpdate {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub organization_id: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveLeadConvert {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedrivePersonsCommand {
    #[command(subcommand)]
    pub action: PipedrivePersonsAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedrivePersonsAction {
    /// List persons with optional filters.
    List(PipedrivePersonList),
    /// Search persons by text.
    Search(PipedrivePersonSearch),
    /// Get one person record.
    Get(PipedriveGetWithLabels),
    /// Get a person with activities, notes, and optional associated email.
    View(PipedriveAssociatedView),
    /// List activities associated with a person.
    Activities(PipedriveAssociatedList),
    /// List notes associated with a person.
    Notes(PipedriveAssociatedList),
    /// List synced email messages associated with a person.
    MailMessages(PipedriveAssociatedList),
    /// Create a person from flags and/or JSON.
    Create(PipedrivePersonWrite),
    /// Update a person from flags and/or JSON.
    Update(PipedrivePersonUpdate),
    /// Delete a person.
    Delete(PipedriveIdArg),
}

#[derive(Debug, Args)]
pub struct PipedrivePersonList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub filter_id: Option<String>,
    #[arg(long)]
    pub ids: Option<String>,
    #[arg(long)]
    pub owner_id: Option<String>,
    #[arg(long)]
    pub org_id: Option<String>,
    #[arg(long)]
    pub deal_id: Option<String>,
    #[arg(long)]
    pub updated_since: Option<String>,
    #[arg(long)]
    pub updated_until: Option<String>,
    #[arg(long)]
    pub sort_by: Option<String>,
    #[arg(long, value_enum)]
    pub sort_direction: Option<PipedriveSortDirection>,
    #[arg(long)]
    pub include_labels: bool,
}

#[derive(Debug, Args)]
pub struct PipedrivePersonSearch {
    #[arg(long)]
    pub term: String,
    #[arg(long)]
    pub fields: Option<String>,
    #[arg(long)]
    pub exact_match: bool,
    #[arg(long)]
    pub organization_id: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct PipedrivePersonWrite {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub org_id: Option<String>,
    #[arg(long)]
    pub email: Option<String>,
    #[arg(long)]
    pub phone: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedrivePersonUpdate {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub org_id: Option<String>,
    #[arg(long)]
    pub email: Option<String>,
    #[arg(long)]
    pub phone: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveOrganizationsCommand {
    #[command(subcommand)]
    pub action: PipedriveOrganizationsAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveOrganizationsAction {
    /// List organizations with optional filters.
    List(PipedriveOrganizationList),
    /// Search organizations by text.
    Search(PipedriveOrganizationSearch),
    /// Get one organization record.
    Get(PipedriveGetWithLabels),
    /// Get an organization with activities, notes, and optional associated email.
    View(PipedriveAssociatedView),
    /// List activities associated with an organization.
    Activities(PipedriveAssociatedList),
    /// List notes associated with an organization.
    Notes(PipedriveAssociatedList),
    /// List synced email messages associated with an organization.
    MailMessages(PipedriveAssociatedList),
    /// Create an organization from flags and/or JSON.
    Create(PipedriveOrganizationWrite),
    /// Update an organization from flags and/or JSON.
    Update(PipedriveOrganizationUpdate),
    /// Delete an organization.
    Delete(PipedriveIdArg),
}

#[derive(Debug, Args)]
pub struct PipedriveOrganizationList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub filter_id: Option<String>,
    #[arg(long)]
    pub ids: Option<String>,
    #[arg(long)]
    pub owner_id: Option<String>,
    #[arg(long)]
    pub updated_since: Option<String>,
    #[arg(long)]
    pub updated_until: Option<String>,
    #[arg(long)]
    pub sort_by: Option<String>,
    #[arg(long, value_enum)]
    pub sort_direction: Option<PipedriveSortDirection>,
    #[arg(long)]
    pub include_labels: bool,
}

#[derive(Debug, Args)]
pub struct PipedriveOrganizationSearch {
    #[arg(long)]
    pub term: String,
    #[arg(long)]
    pub fields: Option<String>,
    #[arg(long)]
    pub exact_match: bool,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct PipedriveOrganizationWrite {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub address: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveOrganizationUpdate {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub address: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveDealsCommand {
    #[command(subcommand)]
    pub action: PipedriveDealsAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveDealsAction {
    /// List deals with optional filters.
    List(PipedriveDealList),
    /// Search deals by text.
    Search(PipedriveDealSearch),
    /// Get one deal record.
    Get(PipedriveGetWithLabels),
    /// Get a deal with activities, notes, and optional associated email.
    View(PipedriveAssociatedView),
    /// List activities associated with a deal.
    Activities(PipedriveAssociatedList),
    /// List notes associated with a deal.
    Notes(PipedriveAssociatedList),
    /// List synced email messages associated with a deal.
    MailMessages(PipedriveAssociatedList),
    /// List updates (including stage transitions) about a deal.
    Flow(PipedriveAssociatedList),
    /// Create a deal from flags and/or JSON.
    Create(PipedriveDealWrite),
    /// Update a deal from flags and/or JSON.
    Update(PipedriveDealUpdate),
    /// Delete a deal.
    Delete(PipedriveIdArg),
}

#[derive(Debug, Args)]
pub struct PipedriveDealList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub filter_id: Option<String>,
    #[arg(long)]
    pub ids: Option<String>,
    #[arg(long)]
    pub owner_id: Option<String>,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub org_id: Option<String>,
    #[arg(long)]
    pub pipeline_id: Option<String>,
    #[arg(long)]
    pub stage_id: Option<String>,
    #[arg(long, value_enum)]
    pub status: Option<PipedriveDealStatus>,
    #[arg(long)]
    pub updated_since: Option<String>,
    #[arg(long)]
    pub updated_until: Option<String>,
    #[arg(long)]
    pub sort_by: Option<String>,
    #[arg(long, value_enum)]
    pub sort_direction: Option<PipedriveSortDirection>,
    #[arg(long)]
    pub include_labels: bool,
}

#[derive(Debug, Args)]
pub struct PipedriveDealSearch {
    #[arg(long)]
    pub term: String,
    #[arg(long)]
    pub fields: Option<String>,
    #[arg(long)]
    pub exact_match: bool,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub organization_id: Option<String>,
    #[arg(long, value_enum)]
    pub status: Option<PipedriveSearchDealStatus>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct PipedriveDealWrite {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub org_id: Option<String>,
    #[arg(long)]
    pub value: Option<f64>,
    #[arg(long)]
    pub currency: Option<String>,
    #[arg(long)]
    pub pipeline_id: Option<String>,
    #[arg(long)]
    pub stage_id: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveDealUpdate {
    pub id: String,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub org_id: Option<String>,
    #[arg(long)]
    pub value: Option<f64>,
    #[arg(long)]
    pub currency: Option<String>,
    #[arg(long)]
    pub pipeline_id: Option<String>,
    #[arg(long)]
    pub stage_id: Option<String>,
    #[arg(long)]
    pub label_ids: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveLabelsCommand {
    #[command(subcommand)]
    pub resource: PipedriveLabelResource,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveLabelResource {
    Leads(PipedriveLeadLabelsCommand),
    Deals(PipedriveLabelListCommand),
    Persons(PipedriveLabelListCommand),
    Organizations(PipedriveLabelListCommand),
}

#[derive(Debug, Args)]
pub struct PipedriveLeadLabelsCommand {
    #[command(subcommand)]
    pub action: PipedriveLeadLabelsAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveLeadLabelsAction {
    List,
    Create(PipedriveLabelCreate),
    Update(PipedriveLabelUpdate),
    Delete(PipedriveIdArg),
}

#[derive(Debug, Args)]
pub struct PipedriveLabelListCommand {
    #[command(subcommand)]
    pub action: PipedriveLabelListAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveLabelListAction {
    List,
}

#[derive(Debug, Args)]
pub struct PipedriveLabelCreate {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub color: String,
}

#[derive(Debug, Args)]
pub struct PipedriveLabelUpdate {
    pub id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub color: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveIdArg {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct PipedriveAssociatedList {
    /// Pipedrive record ID.
    pub id: String,
    /// Maximum associated records to aggregate.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
#[command(
    about = "Manage Apollo CRM, search, outreach, workflow, and analytics APIs",
    long_about = "Manage Apollo people, organizations, contacts, accounts, deals, tasks, calls, sequences, emails, conversations, and analytics.\n\nCreate, update, search, and bulk commands accept `--json <inline|path|->`. Typed flags set common fields and override matching JSON fields.",
    after_help = "Examples:\n  aai-cli apollo people search --title CEO --location Berlin --limit 10\n  aai-cli apollo contacts create --email ada@example.com --first-name Ada --last-name Lovelace\n  aai-cli apollo users me\n  aai-cli apollo request get /users/api_profile"
)]
pub struct ApolloCommand {
    #[command(subcommand)]
    pub resource: ApolloResource,
}

#[derive(Debug, Subcommand)]
pub enum ApolloResource {
    /// Test Apollo API-key authentication.
    Health,
    /// Manage people search and enrichment.
    People(ApolloPeopleCommand),
    /// Manage organization search and enrichment.
    Organizations(ApolloOrganizationsCommand),
    /// Manage Apollo contacts.
    Contacts(ApolloContactsCommand),
    /// Manage Apollo accounts.
    Accounts(ApolloAccountsCommand),
    /// Manage Apollo deals.
    Deals(ApolloDealsCommand),
    /// Manage Apollo tasks.
    Tasks(ApolloTasksCommand),
    /// Manage Apollo call records.
    Calls(ApolloCallsCommand),
    /// List Apollo notes.
    Notes(ApolloNotesCommand),
    /// List users and current API profile.
    Users(ApolloUsersCommand),
    /// List Apollo labels.
    Labels(ApolloListCommand),
    /// List or create Apollo fields.
    Fields(ApolloFieldsCommand),
    /// List Apollo typed custom fields.
    CustomFields(ApolloListCommand),
    /// Inspect Apollo API usage.
    Usage(ApolloUsageCommand),
    /// Poll Apollo webhook results.
    Webhooks(ApolloWebhooksCommand),
    /// Query Apollo analytics reports.
    Analytics(ApolloAnalyticsCommand),
    /// Manage Apollo sequences.
    Sequences(ApolloSequencesCommand),
    /// Manage Apollo outreach emails.
    Emails(ApolloEmailsCommand),
    /// Search Apollo news articles.
    News(ApolloNewsCommand),
    /// Search, inspect, and export Apollo conversations.
    Conversations(ApolloConversationsCommand),
    /// Call an Apollo REST endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
#[command(
    about = "Manage HubSpot CRM, files, events, and conversations APIs",
    long_about = "Manage common HubSpot CRM objects, files, event occurrence reads, behavioral event sends, conversations inboxes, visitor identification, and custom channels.\n\nHubSpot auth failures are returned as structured JSON with endpoint, auth model, likely scopes, provider response, and remediation hints.",
    after_help = "Examples:\n  aai-cli hubspot crm contacts list --limit 25\n  aai-cli hubspot files get 12345\n  aai-cli hubspot conversations inboxes list\n  aai-cli hubspot request get /crm/v3/objects/contacts"
)]
pub struct HubspotCommand {
    #[command(subcommand)]
    pub resource: HubspotResource,
}

#[derive(Debug, Subcommand)]
pub enum HubspotResource {
    /// Test HubSpot token authentication.
    Health,
    /// Read and search HubSpot CRM objects.
    Crm(HubspotCrmCommand),
    /// Read HubSpot files.
    Files(HubspotFilesCommand),
    /// Read event occurrences or send custom behavioral events.
    Events(HubspotEventsCommand),
    /// Read HubSpot conversations APIs and visitor identification tokens.
    Conversations(HubspotConversationsCommand),
    /// Call an uncommon HubSpot REST endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct HubspotCrmCommand {
    #[command(subcommand)]
    pub object: HubspotCrmObjectCommand,
}

#[derive(Debug, Subcommand)]
pub enum HubspotCrmObjectCommand {
    Contacts(HubspotCrmObjectActions),
    Companies(HubspotCrmObjectActions),
    Deals(HubspotCrmObjectActions),
    Tickets(HubspotCrmObjectActions),
}

#[derive(Debug, Args)]
pub struct HubspotCrmObjectActions {
    #[command(subcommand)]
    pub action: HubspotCrmObjectAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotCrmObjectAction {
    List(HubspotListArgs),
    Get(HubspotObjectGet),
    Search(HubspotSearchArgs),
}

#[derive(Debug, Args)]
pub struct HubspotListArgs {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub after: Option<String>,
    /// Comma-separated provider property names.
    #[arg(long)]
    pub properties: Option<String>,
}

#[derive(Debug, Args)]
pub struct HubspotObjectGet {
    pub id: String,
    /// Comma-separated provider property names.
    #[arg(long)]
    pub properties: Option<String>,
    /// Request archived records where supported by HubSpot.
    #[arg(long)]
    pub archived: bool,
}

#[derive(Debug, Args)]
pub struct HubspotSearchArgs {
    /// JSON search body, inline or from a path; use - to read stdin.
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct HubspotFilesCommand {
    #[command(subcommand)]
    pub action: HubspotFilesAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotFilesAction {
    List(HubspotFilesList),
    Get(HubspotFileGet),
}

#[derive(Debug, Args)]
pub struct HubspotFilesList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub after: Option<String>,
    #[arg(long)]
    pub folder_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct HubspotFileGet {
    pub id: String,
    /// Hint that this read may need `files.ui_hidden.read`.
    #[arg(long)]
    pub hidden_or_deleted: bool,
}

#[derive(Debug, Args)]
pub struct HubspotEventsCommand {
    #[command(subcommand)]
    pub resource: HubspotEventsResource,
}

#[derive(Debug, Subcommand)]
pub enum HubspotEventsResource {
    Occurrences(HubspotEventOccurrencesCommand),
    Custom(HubspotCustomEventsCommand),
}

#[derive(Debug, Args)]
pub struct HubspotEventOccurrencesCommand {
    #[command(subcommand)]
    pub action: HubspotEventOccurrencesAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotEventOccurrencesAction {
    List(HubspotEventOccurrencesList),
}

#[derive(Debug, Args)]
pub struct HubspotEventOccurrencesList {
    pub event_type: String,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub after: Option<String>,
}

#[derive(Debug, Args)]
pub struct HubspotCustomEventsCommand {
    #[command(subcommand)]
    pub action: HubspotCustomEventsAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotCustomEventsAction {
    Send(HubspotJsonBody),
}

#[derive(Debug, Args)]
pub struct HubspotConversationsCommand {
    #[command(subcommand)]
    pub resource: HubspotConversationsResource,
}

#[derive(Debug, Subcommand)]
pub enum HubspotConversationsResource {
    Inboxes(HubspotConversationsInboxesCommand),
    Threads(HubspotConversationsThreadsCommand),
    VisitorIdentification(HubspotVisitorIdentificationCommand),
    CustomChannels(HubspotCustomChannelsCommand),
}

#[derive(Debug, Args)]
pub struct HubspotConversationsInboxesCommand {
    #[command(subcommand)]
    pub action: HubspotConversationsInboxesAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotConversationsInboxesAction {
    List(HubspotSimpleList),
}

#[derive(Debug, Args)]
pub struct HubspotConversationsThreadsCommand {
    #[command(subcommand)]
    pub action: HubspotConversationsThreadsAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotConversationsThreadsAction {
    List(HubspotSimpleList),
    Get(IdArg),
}

#[derive(Debug, Args)]
pub struct HubspotVisitorIdentificationCommand {
    #[command(subcommand)]
    pub action: HubspotVisitorIdentificationAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotVisitorIdentificationAction {
    Tokens(HubspotVisitorTokensCommand),
}

#[derive(Debug, Args)]
pub struct HubspotVisitorTokensCommand {
    #[command(subcommand)]
    pub action: HubspotVisitorTokensAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotVisitorTokensAction {
    Create(HubspotJsonBody),
}

#[derive(Debug, Args)]
pub struct HubspotCustomChannelsCommand {
    #[command(subcommand)]
    pub action: HubspotCustomChannelsAction,
}

#[derive(Debug, Subcommand)]
pub enum HubspotCustomChannelsAction {
    List(HubspotSimpleList),
    Get(IdArg),
    Create(HubspotJsonBody),
}

#[derive(Debug, Args)]
pub struct HubspotSimpleList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub after: Option<String>,
}

#[derive(Debug, Args)]
pub struct HubspotJsonBody {
    /// JSON body, inline or from a path; use - to read stdin.
    #[arg(long)]
    pub json: String,
}

#[derive(Debug, Args)]
pub struct ApolloPeopleCommand {
    #[command(subcommand)]
    pub action: ApolloPeopleAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloPeopleAction {
    Search(ApolloSearchArgs),
    Get(ApolloIdArg),
    Enrich(ApolloPeopleEnrich),
    BulkEnrich(ApolloJsonArgs),
}

#[derive(Debug, Args)]
pub struct ApolloPeopleEnrich {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub first_name: Option<String>,
    #[arg(long)]
    pub last_name: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub email: Option<String>,
    #[arg(long)]
    pub organization_name: Option<String>,
    #[arg(long)]
    pub domain: Option<String>,
    #[arg(long)]
    pub id: Option<String>,
    #[arg(long)]
    pub linkedin_url: Option<String>,
    #[arg(long)]
    pub reveal_personal_emails: bool,
    #[arg(long)]
    pub reveal_phone_number: bool,
}

#[derive(Debug, Args)]
pub struct ApolloOrganizationsCommand {
    #[command(subcommand)]
    pub action: ApolloOrganizationsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloOrganizationsAction {
    Search(ApolloSearchArgs),
    Get(ApolloIdArg),
    Enrich(ApolloOrganizationEnrich),
    BulkEnrich(ApolloJsonArgs),
    JobPostings(ApolloPagedIdArg),
}

#[derive(Debug, Args)]
pub struct ApolloOrganizationEnrich {
    #[arg(long)]
    pub domain: Option<String>,
    #[arg(long)]
    pub linkedin_url: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub website: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloContactsCommand {
    #[command(subcommand)]
    pub action: ApolloContactsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloContactsAction {
    Create(ApolloContactWrite),
    Get(ApolloIdArg),
    Search(ApolloSearchArgs),
    Update(ApolloContactUpdate),
    BulkCreate(ApolloJsonArgs),
    BulkUpdate(ApolloJsonArgs),
    UpdateStages(ApolloIdsUpdate),
    UpdateOwners(ApolloIdsUpdate),
    Deals(ApolloIdJsonArgs),
}

#[derive(Debug, Args)]
pub struct ApolloAccountsCommand {
    #[command(subcommand)]
    pub action: ApolloAccountsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloAccountsAction {
    Create(ApolloAccountWrite),
    Get(ApolloIdArg),
    Search(ApolloSearchArgs),
    Update(ApolloAccountUpdate),
    BulkCreate(ApolloJsonArgs),
    BulkUpdate(ApolloJsonArgs),
    UpdateOwners(ApolloIdsUpdate),
    Stages,
}

#[derive(Debug, Args)]
pub struct ApolloDealsCommand {
    #[command(subcommand)]
    pub action: ApolloDealsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloDealsAction {
    Create(ApolloDealWrite),
    List(ApolloSearchArgs),
    Get(ApolloIdArg),
    Update(ApolloDealUpdate),
    Stages,
}

#[derive(Debug, Args)]
pub struct ApolloTasksCommand {
    #[command(subcommand)]
    pub action: ApolloTasksAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloTasksAction {
    Create(ApolloTaskWrite),
    BulkCreate(ApolloJsonArgs),
    Search(ApolloSearchArgs),
}

#[derive(Debug, Args)]
pub struct ApolloCallsCommand {
    #[command(subcommand)]
    pub action: ApolloCallsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloCallsAction {
    Create(ApolloCallWrite),
    Search(ApolloSearchArgs),
    Update(ApolloCallUpdate),
}

#[derive(Debug, Args)]
pub struct ApolloNotesCommand {
    #[command(subcommand)]
    pub action: ApolloNotesAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloNotesAction {
    List(ApolloNotesList),
}

#[derive(Debug, Args)]
pub struct ApolloUsersCommand {
    #[command(subcommand)]
    pub action: ApolloUsersAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloUsersAction {
    List(ApolloSearchArgs),
    Me(ApolloMeArgs),
}

#[derive(Debug, Args)]
pub struct ApolloListCommand {
    #[command(subcommand)]
    pub action: ApolloListAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloListAction {
    List,
}

#[derive(Debug, Args)]
pub struct ApolloFieldsCommand {
    #[command(subcommand)]
    pub action: ApolloFieldsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloFieldsAction {
    List(ApolloFieldsList),
    Create(ApolloFieldWrite),
}

#[derive(Debug, Args)]
pub struct ApolloUsageCommand {
    #[command(subcommand)]
    pub action: ApolloUsageAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloUsageAction {
    Stats,
}

#[derive(Debug, Args)]
pub struct ApolloWebhooksCommand {
    #[command(subcommand)]
    pub action: ApolloWebhooksAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloWebhooksAction {
    Result(ApolloIdArg),
}

#[derive(Debug, Args)]
pub struct ApolloAnalyticsCommand {
    #[command(subcommand)]
    pub action: ApolloAnalyticsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloAnalyticsAction {
    Report(ApolloJsonArgs),
}

#[derive(Debug, Args)]
pub struct ApolloSequencesCommand {
    #[command(subcommand)]
    pub action: ApolloSequencesAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloSequencesAction {
    Search(ApolloSearchArgs),
    Create(ApolloSequenceWrite),
    Update(ApolloSequenceUpdate),
    AddContacts(ApolloSequenceContacts),
    UpdateContactStatus(ApolloSequenceStatus),
    Activate(ApolloIdArg),
    Deactivate(ApolloIdArg),
    Archive(ApolloIdArg),
}

#[derive(Debug, Args)]
pub struct ApolloEmailsCommand {
    #[command(subcommand)]
    pub action: ApolloEmailsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloEmailsAction {
    Draft(ApolloEmailDraft),
    SendNow(ApolloSendNow),
    SendStatus(ApolloJsonArgs),
    Search(ApolloSearchArgs),
    Stats(ApolloIdArg),
    Accounts,
}

#[derive(Debug, Args)]
pub struct ApolloNewsCommand {
    #[command(subcommand)]
    pub action: ApolloNewsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloNewsAction {
    Search(ApolloSearchArgs),
}

#[derive(Debug, Args)]
pub struct ApolloConversationsCommand {
    #[command(subcommand)]
    pub action: ApolloConversationsAction,
}

#[derive(Debug, Subcommand)]
pub enum ApolloConversationsAction {
    Search(ApolloConversationSearch),
    Get(ApolloIdArg),
    Export(ApolloJsonArgs),
    GetExport(ApolloIdArg),
}

#[derive(Debug, Args)]
pub struct ApolloJsonArgs {
    /// JSON object, inline or from a path; use - to read stdin.
    #[arg(long)]
    pub json: Option<String>,
    /// Extra query parameter as key=value. Repeat for multiple parameters.
    #[arg(long = "query")]
    pub query: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ApolloSearchArgs {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub q_keywords: Option<String>,
    #[arg(long)]
    pub q_name: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub location: Option<String>,
    #[arg(long)]
    pub domain: Option<String>,
    #[arg(long)]
    pub sort_by_field: Option<String>,
    #[arg(long)]
    pub sort_ascending: Option<bool>,
}

#[derive(Debug, Args)]
pub struct ApolloIdArg {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ApolloPagedIdArg {
    pub id: String,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long = "query")]
    pub query: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ApolloIdJsonArgs {
    pub id: String,
    #[command(flatten)]
    pub json: ApolloJsonArgs,
}

#[derive(Debug, Args)]
pub struct ApolloIdsUpdate {
    /// Comma-separated Apollo IDs.
    #[arg(long)]
    pub ids: String,
    #[arg(long)]
    pub owner_id: Option<String>,
    #[arg(long)]
    pub stage_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloContactWrite {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub first_name: Option<String>,
    #[arg(long)]
    pub last_name: Option<String>,
    #[arg(long)]
    pub organization_name: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub account_id: Option<String>,
    #[arg(long)]
    pub email: Option<String>,
    #[arg(long)]
    pub website_url: Option<String>,
    #[arg(long)]
    pub contact_stage_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloContactUpdate {
    pub id: String,
    #[command(flatten)]
    pub write: ApolloContactWrite,
}

#[derive(Debug, Args)]
pub struct ApolloAccountWrite {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub domain: Option<String>,
    #[arg(long)]
    pub owner_id: Option<String>,
    #[arg(long)]
    pub account_stage_id: Option<String>,
    #[arg(long)]
    pub phone: Option<String>,
    #[arg(long)]
    pub raw_address: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloAccountUpdate {
    pub id: String,
    #[command(flatten)]
    pub write: ApolloAccountWrite,
}

#[derive(Debug, Args)]
pub struct ApolloDealWrite {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub owner_id: Option<String>,
    #[arg(long)]
    pub account_id: Option<String>,
    #[arg(long)]
    pub amount: Option<f64>,
    #[arg(long)]
    pub opportunity_stage_id: Option<String>,
    #[arg(long)]
    pub closed_date: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloDealUpdate {
    pub id: String,
    #[command(flatten)]
    pub write: ApolloDealWrite,
}

#[derive(Debug, Args)]
pub struct ApolloTaskWrite {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub user_id: Option<String>,
    #[arg(long)]
    pub contact_id: Option<String>,
    #[arg(long = "type")]
    pub task_type: Option<String>,
    #[arg(long)]
    pub priority: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub due_at: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloCallWrite {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub contact_id: Option<String>,
    #[arg(long)]
    pub account_id: Option<String>,
    #[arg(long)]
    pub to_number: Option<String>,
    #[arg(long)]
    pub from_number: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub start_time: Option<String>,
    #[arg(long)]
    pub end_time: Option<String>,
    #[arg(long)]
    pub duration: Option<u64>,
    #[arg(long)]
    pub note: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloCallUpdate {
    pub id: String,
    #[command(flatten)]
    pub write: ApolloCallWrite,
}

#[derive(Debug, Args)]
pub struct ApolloNotesList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub contact_id: Option<String>,
    #[arg(long)]
    pub account_id: Option<String>,
    #[arg(long)]
    pub opportunity_id: Option<String>,
    #[arg(long)]
    pub start_date: Option<String>,
    #[arg(long)]
    pub sort_by_field: Option<String>,
    #[arg(long)]
    pub sort_direction: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloMeArgs {
    #[arg(long)]
    pub include_credit_usage: bool,
}

#[derive(Debug, Args)]
pub struct ApolloFieldsList {
    #[arg(long)]
    pub source: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloFieldWrite {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub label: Option<String>,
    #[arg(long)]
    pub modality: Option<String>,
    #[arg(long = "type")]
    pub field_type: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloSequenceWrite {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub active: Option<bool>,
    #[arg(long)]
    pub user_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloSequenceUpdate {
    pub id: String,
    #[command(flatten)]
    pub write: ApolloSequenceWrite,
}

#[derive(Debug, Args)]
pub struct ApolloSequenceContacts {
    pub id: String,
    #[arg(long)]
    pub contact_ids: String,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub email_account_id: Option<String>,
    #[arg(long)]
    pub email_address: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloSequenceStatus {
    #[arg(long)]
    pub sequence_ids: String,
    #[arg(long)]
    pub contact_ids: String,
    #[arg(long)]
    pub mode: String,
}

#[derive(Debug, Args)]
pub struct ApolloEmailDraft {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub contact_id: Option<String>,
    #[arg(long)]
    pub subject: Option<String>,
    #[arg(long)]
    pub body_html: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloSendNow {
    pub id: String,
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long)]
    pub surface: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApolloConversationSearch {
    #[command(flatten)]
    pub json: ApolloJsonArgs,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub conversation_type: Option<String>,
    #[arg(long)]
    pub account_id: Option<String>,
    #[arg(long)]
    pub sort_by_field: Option<String>,
}

#[derive(Debug, Args)]
#[command(
    about = "Get a complete CRM view of a record",
    long_about = "Returns one JSON object containing the CRM record, associated activities, and associated notes. Use --include-mail to also include synced email history."
)]
pub struct PipedriveAssociatedView {
    /// Pipedrive record ID.
    pub id: String,
    /// Maximum records to aggregate for each associated history category.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    /// Include synced email messages associated with the record.
    #[arg(long)]
    pub include_mail: bool,
    /// Include label details on the primary CRM record.
    #[arg(long)]
    pub include_labels: bool,
}

#[derive(Debug, Args)]
pub struct PipedriveActivitiesCommand {
    #[command(subcommand)]
    pub action: PipedriveActivitiesAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveActivitiesAction {
    /// List activities, optionally filtered by linked CRM records.
    List(Box<PipedriveActivityList>),
    /// Get one activity.
    Get(PipedriveIdArg),
}

#[derive(Debug, Args)]
pub struct PipedriveActivityList {
    /// Maximum activities to aggregate.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub filter_id: Option<String>,
    #[arg(long)]
    pub ids: Option<String>,
    #[arg(long)]
    pub owner_id: Option<String>,
    #[arg(long)]
    pub deal_id: Option<String>,
    #[arg(long)]
    pub lead_id: Option<String>,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub org_id: Option<String>,
    /// Filter by completion state.
    #[arg(long)]
    pub done: Option<bool>,
    #[arg(long)]
    pub updated_since: Option<String>,
    #[arg(long)]
    pub updated_until: Option<String>,
    #[arg(long)]
    pub sort_by: Option<String>,
    #[arg(long, value_enum)]
    pub sort_direction: Option<PipedriveSortDirection>,
    /// Include activity attendees in responses.
    #[arg(long)]
    pub include_attendees: bool,
}

#[derive(Debug, Args)]
pub struct PipedriveNotesCommand {
    #[command(subcommand)]
    pub action: PipedriveNotesAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveNotesAction {
    /// List notes, optionally filtered by linked CRM records.
    List(PipedriveNoteList),
    /// Get one note.
    Get(PipedriveIdArg),
}

#[derive(Debug, Args)]
pub struct PipedriveNoteList {
    /// Maximum notes to aggregate.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub user_id: Option<String>,
    #[arg(long)]
    pub lead_id: Option<String>,
    #[arg(long)]
    pub deal_id: Option<String>,
    #[arg(long)]
    pub person_id: Option<String>,
    #[arg(long)]
    pub org_id: Option<String>,
    #[arg(long)]
    pub sort: Option<String>,
    #[arg(long)]
    pub start_date: Option<String>,
    #[arg(long)]
    pub end_date: Option<String>,
    #[arg(long)]
    pub updated_since: Option<String>,
}

#[derive(Debug, Args)]
pub struct PipedriveMailboxCommand {
    #[command(subcommand)]
    pub resource: PipedriveMailboxResource,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveMailboxResource {
    /// Get individual synced email messages.
    Messages(PipedriveMailboxMessagesCommand),
    /// List and inspect synced email threads.
    Threads(PipedriveMailboxThreadsCommand),
}

#[derive(Debug, Args)]
pub struct PipedriveMailboxMessagesCommand {
    #[command(subcommand)]
    pub action: PipedriveMailboxMessagesAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveMailboxMessagesAction {
    /// Get one synced email message, optionally including its full body.
    Get(PipedriveMailMessageGet),
}

#[derive(Debug, Args)]
pub struct PipedriveMailMessageGet {
    /// Pipedrive mail message ID.
    pub id: String,
    /// Include the full message body, not only metadata and snippet.
    #[arg(long)]
    pub include_body: bool,
}

#[derive(Debug, Args)]
pub struct PipedriveMailboxThreadsCommand {
    #[command(subcommand)]
    pub action: PipedriveMailboxThreadsAction,
}

#[derive(Debug, Subcommand)]
pub enum PipedriveMailboxThreadsAction {
    /// List synced email threads in a mailbox folder.
    List(PipedriveMailThreadList),
    /// Get one synced email thread.
    Get(PipedriveIdArg),
    /// List every message in a synced email thread.
    Messages(PipedriveIdArg),
}

#[derive(Debug, Args)]
pub struct PipedriveMailThreadList {
    /// Mailbox folder to list.
    #[arg(long, value_enum, default_value_t = PipedriveMailFolder::Inbox)]
    pub folder: PipedriveMailFolder,
    /// Maximum threads to aggregate.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum PipedriveMailFolder {
    Inbox,
    Drafts,
    Sent,
    Archive,
}

#[derive(Debug, Args)]
pub struct PipedriveGetWithLabels {
    pub id: String,
    #[arg(long)]
    pub include_labels: bool,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum PipedriveSortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum PipedriveDealStatus {
    Open,
    Won,
    Lost,
    Deleted,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum PipedriveSearchDealStatus {
    Open,
    Won,
    Lost,
}

#[derive(Debug, Subcommand)]
pub enum ListGetAction {
    List(LimitArg),
    Get(IdArg),
}

#[derive(Debug, Args)]
pub struct IdArg {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct NumberArg {
    pub number: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct RepoArg {
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct LimitArg {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct RepoLimitArg {
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct PrListArg {
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    /// Filter by pull request state. GitHub: `open`, `closed`, or `all`.
    /// Bitbucket: `OPEN`, `MERGED`, `DECLINED`, or `SUPERSEDED`. Omitted =
    /// provider default (open). For merged PRs on GitHub use `closed` and keep
    /// those with a non-null `merged_at`.
    #[arg(long)]
    pub state: Option<String>,
    /// Order results. `updated` = most-recently-updated first, so a PR that
    /// merged long after it was opened floats to the top instead of staying
    /// buried in creation order. Omitted = provider default (creation order).
    #[arg(long)]
    pub sort: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use clap::Parser;

    #[test]
    fn github_prs_list_parses_state_flag() {
        let cli = Cli::try_parse_from([
            "aai-cli", "github", "prs", "list", "--owner", "o", "--repo", "r", "--state", "closed",
        ])
        .expect("parse github prs list --state");
        let Command::Github(gh) = cli.command else {
            panic!("expected github command");
        };
        let GithubResource::Prs(prs) = gh.resource else {
            panic!("expected prs resource");
        };
        let GithubPullRequestAction::List(args) = prs.action else {
            panic!("expected list action");
        };
        assert_eq!(args.state.as_deref(), Some("closed"));
    }

    #[test]
    fn bitbucket_prs_list_parses_state_flag() {
        let cli = Cli::try_parse_from([
            "aai-cli",
            "bitbucket",
            "prs",
            "list",
            "--repo",
            "acme/widgets",
            "--state",
            "MERGED",
        ])
        .expect("parse bitbucket prs list --state");
        let Command::Bitbucket(bb) = cli.command else {
            panic!("expected bitbucket command");
        };
        let BitbucketResource::Prs(prs) = bb.resource else {
            panic!("expected prs resource");
        };
        let BitbucketPullRequestAction::List(args) = prs.action else {
            panic!("expected list action");
        };
        assert_eq!(args.state.as_deref(), Some("MERGED"));
    }

    #[test]
    fn drive_files_list_parses_scoping_flags() {
        let cli = Cli::try_parse_from([
            "aai-cli",
            "drive",
            "files",
            "list",
            "--parent",
            "1AbC",
            "--drive-id",
            "0XyZ",
            "--name-contains",
            "runbook",
            "--include-trashed",
            "--limit",
            "10",
        ])
        .expect("parse drive files list");
        let Command::Drive(drive) = cli.command else {
            panic!("expected drive command");
        };
        let DriveResource::Files(files) = drive.resource else {
            panic!("expected files resource");
        };
        let DriveFilesAction::List(args) = files.action else {
            panic!("expected list action");
        };
        assert_eq!(args.parent.as_deref(), Some("1AbC"));
        assert_eq!(args.drive_id.as_deref(), Some("0XyZ"));
        assert_eq!(args.name_contains.as_deref(), Some("runbook"));
        assert!(args.include_trashed);
        assert_eq!(args.limit, 10);
    }

    #[test]
    fn drive_files_download_requires_an_output_path() {
        // Content must never land on stdout, so there is no default for --output.
        Cli::try_parse_from(["aai-cli", "drive", "files", "download", "1AbC"])
            .expect_err("download without --output must fail");
    }

    #[test]
    fn drive_exposes_reads_and_only_the_upload_write() {
        let mut command = Cli::command();
        let drive = command.find_subcommand_mut("drive").expect("drive command");
        let mut help = Vec::new();
        drive.write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();

        for resource in ["files", "folders", "drives", "permissions", "about"] {
            assert!(help.contains(resource), "drive help lacks {resource}");
        }

        // Upload is the only write. Everything that would mutate Drive state — most
        // importantly sharing — has no command at all, so an agent cannot reach it.
        for absent in [
            vec!["drive", "files", "delete", "1AbC"],
            vec!["drive", "files", "trash", "1AbC"],
            vec!["drive", "files", "update", "1AbC"],
            vec!["drive", "permissions", "create", "1AbC"],
            vec!["drive", "permissions", "delete", "1AbC", "1XyZ"],
        ] {
            let mut args = vec!["aai-cli"];
            args.extend(absent.iter().copied());
            Cli::try_parse_from(args)
                .expect_err(&format!("{absent:?} must not be a supported command"));
        }
    }

    #[test]
    fn pipedrive_history_commands_are_discoverable_in_help() {
        let mut command = Cli::command();
        let pipedrive = command
            .find_subcommand_mut("pipedrive")
            .expect("pipedrive command");
        let mut help = Vec::new();
        pipedrive.write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();

        assert!(help.contains("synced email"));
        assert!(help.contains("deals view"));
        assert!(help.contains("mailbox messages get"));
        assert!(help.contains("activities"));
        assert!(help.contains("notes"));
    }

    #[test]
    fn jira_ideas_commands_are_discoverable_in_help() {
        let mut command = Cli::command();
        let jira = command.find_subcommand_mut("jira").expect("jira command");
        let mut help = Vec::new();
        jira.write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();

        assert!(help.contains("ideas"));
        assert!(help.contains("Product Discovery"));
    }

    #[test]
    fn jira_ideas_parse_url_accepts_a_view_url() {
        let cli = Cli::try_parse_from([
            "aai-cli",
            "jira",
            "ideas",
            "parse-url",
            "https://example.atlassian.net/jira/polaris/projects/MDP/ideas/view/123",
        ])
        .expect("parse jira ideas parse-url");
        let Command::Jira(jira) = cli.command else {
            panic!("expected jira command");
        };
        let JiraResource::Ideas(ideas) = jira.resource else {
            panic!("expected ideas resource");
        };
        let JiraIdeasAction::ParseUrl(args) = ideas.action else {
            panic!("expected parse-url action");
        };
        assert!(args.url.contains("/projects/MDP/"));
    }

    #[test]
    fn jira_ideas_transition_parses_name_and_json() {
        let cli = Cli::try_parse_from([
            "aai-cli",
            "jira",
            "ideas",
            "transitions",
            "perform",
            "MDP-42",
            "--transition",
            "Research",
            "--json",
            r#"{"fields":{"customfield_10011":{"id":"10"}}}"#,
        ])
        .expect("parse jira idea transition");
        let Command::Jira(jira) = cli.command else {
            panic!("expected jira command");
        };
        let JiraResource::Ideas(ideas) = jira.resource else {
            panic!("expected ideas resource");
        };
        let JiraIdeasAction::Transitions(command) = ideas.action else {
            panic!("expected transitions action");
        };
        let JiraIdeaTransitionsAction::Perform(args) = command.action else {
            panic!("expected transition perform action");
        };
        assert_eq!(args.id, "MDP-42");
        assert_eq!(args.transition, "Research");
        assert!(args.json.is_some());
    }

    #[test]
    fn config_profile_commands_are_discoverable_in_help() {
        let mut command = Cli::command();
        let config = command
            .find_subcommand_mut("config")
            .expect("config command");
        let mut help = Vec::new();
        config.write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();

        assert!(help.contains("profiles"));
        assert!(help.contains("default-profile"));
        assert!(help.contains("without exposing credentials"));
    }

    #[test]
    fn generic_request_is_discoverable_for_http_services() {
        for service in [
            "jira",
            "confluence",
            "bitbucket",
            "github",
            "email",
            "calendar",
            "pipedrive",
            "slack",
        ] {
            let mut command = Cli::command();
            let service_command = command
                .find_subcommand_mut(service)
                .expect("service command");
            let mut help = Vec::new();
            service_command.write_long_help(&mut help).unwrap();
            let help = String::from_utf8(help).unwrap();
            assert!(help.contains("request"), "{service} help lacks request");
        }
    }

    #[test]
    fn microsoft_typed_resources_are_discoverable_and_parse() {
        let mut command = Cli::command();
        let microsoft = command
            .find_subcommand_mut("microsoft")
            .expect("microsoft command");
        let mut help = Vec::new();
        microsoft.write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();

        for resource in [
            "auth",
            "files",
            "mail",
            "calendar",
            "contacts",
            "sharepoint",
            "teams",
            "todo",
            "planner",
            "request",
        ] {
            assert!(help.contains(resource), "microsoft help lacks {resource}");
        }

        Cli::try_parse_from([
            "aai-cli",
            "microsoft",
            "planner",
            "tasks",
            "update",
            "task-id",
            "--etag",
            "etag-value",
            "--json",
            r#"{"title":"updated"}"#,
        ])
        .expect("parse planner task update");
    }

    #[test]
    fn pagination_contract_is_discoverable_in_top_level_help() {
        let mut help = Vec::new();
        Cli::command().write_long_help(&mut help).unwrap();
        let help = String::from_utf8(help).unwrap();
        assert!(help.contains("_aai.pagination"));
        assert!(help.contains("retrieving more results"));
    }
}

#[derive(Debug, Subcommand)]
pub enum PullRequestAction {
    List(RepoLimitArg),
    Get(NumberArg),
    Create(PullRequestCreate),
    Delete(NumberArg),
}

#[derive(Debug, Subcommand)]
pub enum GithubPullRequestAction {
    List(PrListArg),
    Get(NumberArg),
    Create(PullRequestCreate),
    Delete(NumberArg),
    Close(NumberArg),
    Decline(NumberArg),
    Diff(GithubPrDiff),
    Files(GithubPrFiles),
    Commits(GithubPrCommits),
    Timeline(GithubPrTimeline),
    Comments(GithubPrCommentsCommand),
    #[command(name = "review-comments")]
    ReviewComments(GithubPrReviewCommentsCommand),
    Reviews(GithubPrReviewsCommand),
}

#[derive(Debug, Args)]
pub struct GithubPrDiff {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubPrFiles {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct GithubPrCommits {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct GithubPrTimeline {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct GithubPrCommentsCommand {
    #[command(subcommand)]
    pub action: GithubPrCommentAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubPrCommentAction {
    List(GithubPrCommentList),
    Get(GithubPrCommentGet),
    Create(GithubPrCommentWrite),
    Update(GithubPrCommentWrite),
    Delete(GithubPrCommentGet),
}

#[derive(Debug, Args)]
pub struct GithubPrCommentList {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct GithubPrCommentGet {
    pub pr: u64,
    pub comment: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubPrCommentWrite {
    pub pr: u64,
    #[arg(long)]
    pub comment: Option<u64>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubPrReviewCommentsCommand {
    #[command(subcommand)]
    pub action: GithubPrReviewCommentAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubPrReviewCommentAction {
    List(GithubPrReviewCommentList),
    Get(GithubPrReviewCommentGet),
    Create(GithubPrReviewCommentCreate),
    Update(GithubPrReviewCommentUpdate),
    Delete(GithubPrReviewCommentGet),
}

#[derive(Debug, Args)]
pub struct GithubPrReviewCommentList {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct GithubPrReviewCommentGet {
    pub pr: u64,
    pub comment: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubPrReviewCommentCreate {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long)]
    pub line: Option<u64>,
    #[arg(long)]
    pub side: Option<String>,
    #[arg(long = "start-line")]
    pub start_line: Option<u64>,
    #[arg(long = "start-side")]
    pub start_side: Option<String>,
    #[arg(long = "commit-id")]
    pub commit_id: Option<String>,
    #[arg(long = "in-reply-to")]
    pub in_reply_to: Option<u64>,
}

#[derive(Debug, Args)]
pub struct GithubPrReviewCommentUpdate {
    pub pr: u64,
    #[arg(long)]
    pub comment: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubPrReviewsCommand {
    #[command(subcommand)]
    pub action: GithubPrReviewAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubPrReviewAction {
    List(GithubPrReviewList),
    Get(GithubPrReviewGet),
    Create(GithubPrReviewCreate),
}

#[derive(Debug, Args)]
pub struct GithubPrReviewList {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct GithubPrReviewGet {
    pub pr: u64,
    pub review: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubPrReviewCreate {
    pub pr: u64,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub event: Option<String>,
    #[arg(long = "commit-id")]
    pub commit_id: Option<String>,
    #[arg(long = "comments-json")]
    pub comments_json: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubBranchesCommand {
    #[command(subcommand)]
    pub action: GithubBranchesAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubBranchesAction {
    List(GithubBranchList),
    Get(GithubBranchGet),
}

#[derive(Debug, Args)]
pub struct GithubBranchList {
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long, conflicts_with = "name_prefix")]
    pub name_contains: Option<String>,
    #[arg(long, conflicts_with = "name_contains")]
    pub name_prefix: Option<String>,
    #[arg(long)]
    pub protected: Option<bool>,
}

#[derive(Debug, Args)]
pub struct GithubBranchGet {
    pub name: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Debug, Args)]
pub struct GithubSourceCommand {
    #[command(subcommand)]
    pub action: GithubSourceAction,
}

#[derive(Debug, Subcommand)]
pub enum GithubSourceAction {
    Get(GithubSourceGet),
    History(GithubSourceHistory),
}

#[derive(Debug, Args)]
pub struct GithubSourceGet {
    pub commit: String,
    pub path: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, conflicts_with = "meta")]
    pub output: Option<String>,
    #[arg(long, conflicts_with = "output")]
    pub meta: bool,
}

#[derive(Debug, Args)]
pub struct GithubSourceHistory {
    pub commit: String,
    pub path: String,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct SheetsCommand {
    #[command(subcommand)]
    pub resource: SheetsResource,
}

#[derive(Debug, Subcommand)]
pub enum SheetsResource {
    /// Create spreadsheets, discover them in Drive, or get metadata for a specific one.
    Spreadsheets(SpreadsheetsCommand),
    /// Add, delete, or rename the sheet tabs inside a spreadsheet.
    Sheets(SheetsTabCommand),
    /// Read, write, or clear cell values in a spreadsheet range.
    Values(ValuesCommand),
}

#[derive(Debug, Args)]
pub struct SpreadsheetsCommand {
    #[command(subcommand)]
    pub action: SpreadsheetsAction,
}

#[derive(Debug, Subcommand)]
pub enum SpreadsheetsAction {
    /// Create a new spreadsheet in Drive.
    Create(SpreadsheetsCreateArgs),
    /// List all Google Sheets spreadsheets in Drive.
    List(SpreadsheetsListArgs),
    /// Get spreadsheet metadata including all sheet tab names and IDs.
    Get(SpreadsheetsGetArgs),
}

#[derive(Debug, Args)]
pub struct SpreadsheetsCreateArgs {
    /// Title for the new spreadsheet.
    pub title: String,
    /// Comma-separated tab names to create, e.g. 'Q1,Q2,Q3'. Defaults to a single 'Sheet1'.
    #[arg(long)]
    pub sheets: Option<String>,
}

#[derive(Debug, Args)]
pub struct SpreadsheetsListArgs {
    /// Pagination token from a previous response to fetch the next page.
    #[arg(long)]
    pub page_token: Option<String>,
}

#[derive(Debug, Args)]
pub struct SheetsTabCommand {
    #[command(subcommand)]
    pub action: SheetsTabAction,
}

#[derive(Debug, Subcommand)]
pub enum SheetsTabAction {
    /// Add a new tab to an existing spreadsheet.
    Add(SheetsTabAddArgs),
    /// Delete a tab. The spreadsheet's last remaining tab cannot be deleted.
    Delete(SheetsTabDeleteArgs),
    /// Rename a tab. Formulas referencing the old name are updated by Google.
    Rename(SheetsTabRenameArgs),
}

#[derive(Debug, Args)]
pub struct SheetsTabAddArgs {
    pub spreadsheet_id: String,
    /// Title for the new tab.
    pub title: String,
}

#[derive(Debug, Args)]
pub struct SheetsTabDeleteArgs {
    pub spreadsheet_id: String,
    /// Title of the tab to delete.
    pub title: String,
}

#[derive(Debug, Args)]
pub struct SheetsTabRenameArgs {
    pub spreadsheet_id: String,
    /// Current title of the tab.
    pub title: String,
    /// New title for the tab.
    pub new_title: String,
}

#[derive(Debug, Args)]
pub struct SpreadsheetsGetArgs {
    pub spreadsheet_id: String,
}

#[derive(Debug, Args)]
pub struct ValuesCommand {
    #[command(subcommand)]
    pub action: ValuesAction,
}

#[derive(Debug, Subcommand)]
pub enum ValuesAction {
    /// Read cell values from a range (e.g. 'Sheet1'!A1:D5).
    Get(ValuesGetArgs),
    /// Write cell values to a range.
    Update(ValuesUpdateArgs),
    /// Clear cell values from a range (formatting is preserved).
    Clear(ValuesClearArgs),
}

#[derive(Debug, Args)]
pub struct ValuesGetArgs {
    pub spreadsheet_id: String,
    /// A1 notation range, e.g. 'Sheet1'!A1:D5 or 'Inventory'!2:2
    pub range: String,
}

#[derive(Debug, Args)]
pub struct ValuesUpdateArgs {
    pub spreadsheet_id: String,
    /// A1 notation range, e.g. 'Sheet1'!A1:D5
    pub range: String,
    /// JSON array of arrays: [["A1","B1"],["A2","B2"]]
    #[arg(long)]
    pub values: String,
}

#[derive(Debug, Args)]
pub struct ValuesClearArgs {
    pub spreadsheet_id: String,
    /// A1 notation range, e.g. 'Sheet1'!A1:D5
    pub range: String,
}

#[derive(Debug, Args)]
#[command(
    after_help = "Examples:\n  aai-cli drive files list --parent 1AbC... --limit 20\n  aai-cli drive files download 1XyZ... --output ./report.docx\n  aai-cli drive files upload ./notes.md --parent 1AbC...\n  aai-cli drive permissions list 1XyZ..."
)]
pub struct DriveCommand {
    #[command(subcommand)]
    pub resource: DriveResource,
}

#[derive(Debug, Subcommand)]
pub enum DriveResource {
    /// List and read file metadata, download content, and upload new files.
    Files(DriveFilesCommand),
    /// List and read folders. A folder is a file with the Drive folder MIME type.
    Folders(DriveFoldersCommand),
    /// List and read shared drives.
    Drives(DriveDrivesCommand),
    /// Read who a file is shared with. Sharing itself is deliberately not writable.
    Permissions(DrivePermissionsCommand),
    /// Read storage quota and the conversions Drive supports.
    About(DriveAboutCommand),
}

#[derive(Debug, Args)]
pub struct DriveFilesCommand {
    #[command(subcommand)]
    pub action: DriveFilesAction,
}

#[derive(Debug, Subcommand)]
pub enum DriveFilesAction {
    /// List file metadata. Shared-drive content is included.
    List(DriveFilesListArgs),
    /// Get metadata for one file.
    Get(DriveFilesGetArgs),
    /// Download file content to --output. Google-native docs are exported instead.
    Download(DriveFilesDownloadArgs),
    /// Upload a local file to Drive as a new file.
    Upload(DriveFilesUploadArgs),
}

#[derive(Debug, Args)]
pub struct DriveFilesListArgs {
    /// Only list direct children of this folder ID.
    #[arg(long)]
    pub parent: Option<String>,
    /// Restrict the search to one shared drive ID.
    #[arg(long)]
    pub drive_id: Option<String>,
    /// Only list files whose name contains this text.
    #[arg(long)]
    pub name_contains: Option<String>,
    /// Only list files with this exact MIME type.
    #[arg(long)]
    pub mime_type: Option<String>,
    /// Raw Drive query clause, ANDed with the other filters. See Drive's search-for-files guide.
    #[arg(long)]
    pub q: Option<String>,
    /// Drive sort spec, e.g. 'modifiedTime desc' or 'folder,name'.
    #[arg(long)]
    pub order_by: Option<String>,
    /// Per-file field projection, e.g. 'id,name,size'. Replaces the default projection.
    #[arg(long)]
    pub fields: Option<String>,
    /// Include trashed files. They are excluded by default.
    #[arg(long)]
    pub include_trashed: bool,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    /// Continue from a previous response's nextPageToken.
    #[arg(long)]
    pub page_token: Option<String>,
}

#[derive(Debug, Args)]
pub struct DriveFilesGetArgs {
    pub file_id: String,
    /// Field projection. Replaces the default projection.
    #[arg(long)]
    pub fields: Option<String>,
}

#[derive(Debug, Args)]
pub struct DriveFilesDownloadArgs {
    pub file_id: String,
    /// Path to write the file content to. Content never goes to stdout.
    #[arg(long)]
    pub output: String,
    /// Export target MIME type for Google-native docs. Ignored for ordinary blobs.
    #[arg(long)]
    pub mime_type: Option<String>,
}

#[derive(Debug, Args)]
pub struct DriveFilesUploadArgs {
    /// Path to the local file to upload.
    pub file: String,
    /// Name for the created Drive file. Defaults to the local file name.
    #[arg(long)]
    pub name: Option<String>,
    /// Folder ID (or shared drive ID) to create the file in.
    #[arg(long)]
    pub parent: Option<String>,
    /// Content MIME type. Guessed from the file extension when omitted.
    #[arg(long)]
    pub mime_type: Option<String>,
    /// Description stored on the Drive file.
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct DriveFoldersCommand {
    #[command(subcommand)]
    pub action: DriveFoldersAction,
}

#[derive(Debug, Subcommand)]
pub enum DriveFoldersAction {
    /// List folders, optionally scoped to a parent folder or shared drive.
    List(DriveFoldersListArgs),
    /// Get metadata for one folder.
    Get(DriveFoldersGetArgs),
}

#[derive(Debug, Args)]
pub struct DriveFoldersListArgs {
    /// Only list folders directly inside this folder ID.
    #[arg(long)]
    pub parent: Option<String>,
    /// Restrict the search to one shared drive ID.
    #[arg(long)]
    pub drive_id: Option<String>,
    /// Only list folders whose name contains this text.
    #[arg(long)]
    pub name_contains: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    /// Continue from a previous response's nextPageToken.
    #[arg(long)]
    pub page_token: Option<String>,
}

#[derive(Debug, Args)]
pub struct DriveFoldersGetArgs {
    pub folder_id: String,
    /// Field projection. Replaces the default projection.
    #[arg(long)]
    pub fields: Option<String>,
}

#[derive(Debug, Args)]
pub struct DriveDrivesCommand {
    #[command(subcommand)]
    pub action: DriveDrivesAction,
}

#[derive(Debug, Subcommand)]
pub enum DriveDrivesAction {
    /// List shared drives the caller is a member of.
    List(DriveDrivesListArgs),
    /// Get metadata for one shared drive.
    Get(DriveDrivesGetArgs),
}

#[derive(Debug, Args)]
pub struct DriveDrivesListArgs {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    /// Continue from a previous response's nextPageToken.
    #[arg(long)]
    pub page_token: Option<String>,
}

#[derive(Debug, Args)]
pub struct DriveDrivesGetArgs {
    pub drive_id: String,
}

#[derive(Debug, Args)]
pub struct DrivePermissionsCommand {
    #[command(subcommand)]
    pub action: DrivePermissionsAction,
}

#[derive(Debug, Subcommand)]
pub enum DrivePermissionsAction {
    /// List the permissions on a file: who can see it and with what role.
    List(DrivePermissionsListArgs),
    /// Get one permission by ID.
    Get(DrivePermissionsGetArgs),
}

#[derive(Debug, Args)]
pub struct DrivePermissionsListArgs {
    pub file_id: String,
    #[arg(long, default_value_t = 100)]
    pub limit: u32,
    /// Continue from a previous response's nextPageToken.
    #[arg(long)]
    pub page_token: Option<String>,
}

#[derive(Debug, Args)]
pub struct DrivePermissionsGetArgs {
    pub file_id: String,
    pub permission_id: String,
}

#[derive(Debug, Args)]
pub struct DriveAboutCommand {
    #[command(subcommand)]
    pub action: DriveAboutAction,
}

#[derive(Debug, Subcommand)]
pub enum DriveAboutAction {
    /// Get the caller's storage quota and Drive's supported import/export conversions.
    Get,
}

#[derive(Debug, Args)]
pub struct ExcelCommand {
    #[command(subcommand)]
    pub resource: ExcelResource,
}

#[derive(Debug, Subcommand)]
pub enum ExcelResource {
    /// Create a new spreadsheet file (.xlsx or .csv/.tsv).
    Workbook(ExcelWorkbookCommand),
    /// Inspect, add, delete, or rename the sheet tabs in a workbook.
    Sheets(ExcelSheetsCommand),
    /// Read, write, or clear cell values in a workbook range.
    Values(ExcelValuesCommand),
}

#[derive(Debug, Args)]
pub struct ExcelWorkbookCommand {
    #[command(subcommand)]
    pub action: ExcelWorkbookAction,
}

#[derive(Debug, Subcommand)]
pub enum ExcelWorkbookAction {
    /// Create a new empty spreadsheet file.
    Create(ExcelWorkbookCreateArgs),
}

#[derive(Debug, Args)]
pub struct ExcelWorkbookCreateArgs {
    /// Path to write the new file to (.xlsx, or .csv/.tsv for a delimited file).
    pub file: PathBuf,
    /// Comma-separated sheet tab names (.xlsx only). Defaults to a single "Sheet1".
    #[arg(long)]
    pub sheets: Option<String>,
    /// Overwrite the file if it already exists.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ExcelSheetsCommand {
    #[command(subcommand)]
    pub action: ExcelSheetsAction,
}

#[derive(Debug, Subcommand)]
pub enum ExcelSheetsAction {
    /// List every sheet tab in the workbook, with its used range.
    List(ExcelSheetsListArgs),
    /// Add a new empty sheet tab to the end of the workbook (.xlsx only).
    Add(ExcelSheetsAddArgs),
    /// Delete a sheet tab (.xlsx only). Refused for the last tab, or if formulas reference it.
    Delete(ExcelSheetsDeleteArgs),
    /// Rename a sheet tab (.xlsx only). Refused if formulas reference the old name.
    Rename(ExcelSheetsRenameArgs),
}

#[derive(Debug, Args)]
pub struct ExcelSheetsListArgs {
    /// Path to the spreadsheet file.
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct ExcelSheetsAddArgs {
    /// Path to the .xlsx workbook.
    pub file: PathBuf,
    /// Title for the new sheet tab.
    pub title: String,
    /// Write even if the workbook has features a rewrite would drop.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ExcelSheetsDeleteArgs {
    /// Path to the .xlsx workbook.
    pub file: PathBuf,
    /// Title of the sheet tab to delete.
    pub title: String,
    /// Delete even if formulas reference the tab, or the workbook has features a rewrite would drop.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ExcelSheetsRenameArgs {
    /// Path to the .xlsx workbook.
    pub file: PathBuf,
    /// Current title of the sheet tab.
    pub title: String,
    /// New title for the sheet tab.
    pub new_title: String,
    /// Rename even if formulas reference the old name, or the workbook has features a rewrite would drop.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ExcelValuesCommand {
    #[command(subcommand)]
    pub action: ExcelValuesAction,
}

#[derive(Debug, Subcommand)]
pub enum ExcelValuesAction {
    /// Read cell values from a range (e.g. 'Sheet1'!A1:D5).
    Get(ExcelValuesGetArgs),
    /// Write cell values to a range.
    Update(ExcelValuesUpdateArgs),
    /// Clear cell values from a range (formatting is preserved).
    Clear(ExcelValuesClearArgs),
}

#[derive(Debug, Args)]
pub struct ExcelValuesGetArgs {
    /// Path to the spreadsheet file.
    pub file: PathBuf,
    /// A1 notation range, e.g. 'Sheet1'!A1:D5. A bare sheet name reads its used range.
    pub range: String,
}

#[derive(Debug, Args)]
pub struct ExcelValuesUpdateArgs {
    /// Path to the spreadsheet file.
    pub file: PathBuf,
    /// A1 notation top-left anchor or full range, e.g. 'Sheet1'!A1
    pub range: String,
    /// JSON array of arrays: [["A1","B1"],["A2","B2"]]. null leaves a cell empty.
    #[arg(long)]
    pub values: String,
    /// Write even when the workbook holds features a rewrite cannot preserve
    /// (charts, pivot tables, form controls, drawings, external links).
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ExcelValuesClearArgs {
    /// Path to the spreadsheet file.
    pub file: PathBuf,
    /// A1 notation range, e.g. 'Sheet1'!A1:D5
    pub range: String,
    /// Write even when the workbook holds features a rewrite cannot preserve
    /// (charts, pivot tables, form controls, drawings, external links).
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct PullRequestCreate {
    #[arg(long)]
    pub json: Option<String>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub source: Option<String>,
    #[arg(long)]
    pub destination: Option<String>,
    #[arg(long)]
    pub head: Option<String>,
    #[arg(long)]
    pub base: Option<String>,
}

#[derive(Debug, Args)]
pub struct SlackCommand {
    #[command(subcommand)]
    pub resource: SlackResource,
}

#[derive(Debug, Subcommand)]
pub enum SlackResource {
    /// List and inspect Slack channels.
    Channels(SlackChannelsCommand),
    /// List files shared in a Slack channel.
    Files(SlackFilesCommand),
    /// List bookmarks on a Slack channel.
    Bookmarks(SlackBookmarksCommand),
    /// Extract links shared in a Slack channel's message history.
    Links(SlackLinksCommand),
    /// Work with a Slack channel's canvas.
    Canvas(SlackCanvasCommand),
    /// Call an uncommon Slack Web API method with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct SlackChannelsCommand {
    #[command(subcommand)]
    pub action: SlackChannelsAction,
}

#[derive(Debug, Subcommand)]
pub enum SlackChannelsAction {
    /// List channels visible to the bot token.
    List(SlackChannelList),
    /// Get one channel's metadata, including a convenience canvas_id if present.
    Get(SlackChannelIdArg),
}

#[derive(Debug, Args)]
pub struct SlackChannelList {
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    /// Comma-separated conversation types passed to conversations.list.
    #[arg(long, default_value = "public_channel,private_channel")]
    pub types: String,
}

#[derive(Debug, Args)]
pub struct SlackChannelIdArg {
    pub channel_id: String,
}

#[derive(Debug, Args)]
pub struct SlackChannelAssociatedList {
    pub channel_id: String,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct SlackFilesCommand {
    #[command(subcommand)]
    pub action: SlackFilesAction,
}

#[derive(Debug, Subcommand)]
pub enum SlackFilesAction {
    /// List files shared in a channel.
    List(SlackChannelAssociatedList),
    /// Download a file's content to a local file.
    Download(SlackFileDownload),
}

#[derive(Debug, Args)]
pub struct SlackFileDownload {
    pub file_id: String,
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct SlackBookmarksCommand {
    #[command(subcommand)]
    pub action: SlackBookmarksAction,
}

#[derive(Debug, Subcommand)]
pub enum SlackBookmarksAction {
    /// List a channel's bookmarks (Slack caps these at 100 per channel; unpaginated).
    List(SlackChannelIdArg),
}

#[derive(Debug, Args)]
pub struct SlackLinksCommand {
    #[command(subcommand)]
    pub action: SlackLinksAction,
}

#[derive(Debug, Subcommand)]
pub enum SlackLinksAction {
    /// Extract links from a channel's message history.
    List(SlackChannelAssociatedList),
}

#[derive(Debug, Args)]
pub struct SlackCanvasCommand {
    #[command(subcommand)]
    pub action: SlackCanvasAction,
}

#[derive(Debug, Subcommand)]
pub enum SlackCanvasAction {
    /// Download a channel's canvas content to a local file.
    Download(SlackCanvasDownload),
}

#[derive(Debug, Args)]
pub struct SlackCanvasDownload {
    pub channel_id: String,
    #[arg(long)]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct OpenpanelCommand {
    #[command(subcommand)]
    pub resource: OpenpanelResource,
}

#[derive(Debug, Subcommand)]
pub enum OpenpanelResource {
    /// List and inspect OpenPanel projects (requires a root client).
    Projects(OpenpanelProjectsCommand),
    /// Export raw tracked events (requires a read or root client).
    Events(OpenpanelEventsCommand),
    /// Query aggregated analytics: metrics, top pages, referrers, devices, geo.
    Insights(OpenpanelInsightsCommand),
    /// Search user profiles and inspect a single profile's recent events.
    Profiles(OpenpanelProfilesCommand),
    /// Call an uncommon OpenPanel endpoint with profile authentication.
    Request(GenericRequest),
}

#[derive(Debug, Args)]
pub struct OpenpanelProjectsCommand {
    #[command(subcommand)]
    pub action: OpenpanelProjectsAction,
}

#[derive(Debug, Subcommand)]
pub enum OpenpanelProjectsAction {
    /// List every project in the client's organization.
    List,
    /// Get a single project by ID.
    Get(OpenpanelProjectId),
}

#[derive(Debug, Args)]
pub struct OpenpanelProjectId {
    pub project_id: String,
}

#[derive(Debug, Args)]
pub struct OpenpanelEventsCommand {
    #[command(subcommand)]
    pub action: OpenpanelEventsAction,
}

#[derive(Debug, Subcommand)]
pub enum OpenpanelEventsAction {
    /// Export a paginated list of raw tracked events.
    Export(OpenpanelEventsExport),
}

#[derive(Debug, Args)]
pub struct OpenpanelEventsExport {
    /// Project to export from. Required unless the client is scoped to a single project.
    #[arg(long)]
    pub project_id: Option<String>,
    /// Event name to filter by. Repeat for multiple names.
    #[arg(long = "event")]
    pub event: Vec<String>,
    #[arg(long)]
    pub profile_id: Option<String>,
    /// Inclusive range start, any format accepted by the JS Date constructor (e.g. 2024-01-01).
    #[arg(long)]
    pub start: Option<String>,
    /// Inclusive range end, same format as --start.
    #[arg(long)]
    pub end: Option<String>,
    /// Total events to return across pages.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    /// Comma-separated extra data to attach: profile, meta.
    #[arg(long)]
    pub includes: Option<String>,
    /// JSON array of chart event filters, passed through to the provider as-is.
    #[arg(long)]
    pub filters: Option<String>,
}

#[derive(Debug, Args)]
pub struct OpenpanelInsightsCommand {
    #[command(subcommand)]
    pub action: OpenpanelInsightsAction,
}

#[derive(Debug, Subcommand)]
pub enum OpenpanelInsightsAction {
    /// Aggregated visitors, sessions, and bounce rate for a date range.
    Metrics(OpenpanelDateRangeArgs),
    /// Top pages by pageviews for a date range.
    Pages(OpenpanelBreakdownListArgs),
    /// Top referrer/UTM breakdown for a date range.
    Referrers(OpenpanelReferrersArgs),
    /// Top device/browser/OS breakdown for a date range.
    Devices(OpenpanelDevicesArgs),
    /// Top country/region/city breakdown for a date range.
    Geo(OpenpanelGeoArgs),
}

#[derive(Debug, Args)]
pub struct OpenpanelDateRangeArgs {
    /// Project to query. Falls back to profile.project_id.
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long)]
    pub start_date: Option<String>,
    #[arg(long)]
    pub end_date: Option<String>,
    /// Relative range preset (e.g. 7d, 30d, 3m), used when --start-date is omitted.
    #[arg(long)]
    pub range: Option<String>,
    /// JSON array of chart event filters, passed through to the provider as-is.
    #[arg(long)]
    pub filters: Option<String>,
}

#[derive(Debug, Args)]
pub struct OpenpanelBreakdownListArgs {
    #[command(flatten)]
    pub date_range: OpenpanelDateRangeArgs,
    #[arg(long)]
    pub cursor: Option<u64>,
    #[arg(long, default_value_t = 10)]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct OpenpanelReferrersArgs {
    #[command(flatten)]
    pub list: OpenpanelBreakdownListArgs,
    #[arg(long, value_enum, default_value_t = OpenpanelReferrerBreakdown::ReferrerName)]
    pub breakdown: OpenpanelReferrerBreakdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OpenpanelReferrerBreakdown {
    Referrer,
    ReferrerName,
    ReferrerType,
    UtmSource,
    UtmMedium,
    UtmCampaign,
    UtmTerm,
    UtmContent,
}

#[derive(Debug, Args)]
pub struct OpenpanelDevicesArgs {
    #[command(flatten)]
    pub list: OpenpanelBreakdownListArgs,
    #[arg(long, value_enum, default_value_t = OpenpanelDeviceBreakdown::Device)]
    pub breakdown: OpenpanelDeviceBreakdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OpenpanelDeviceBreakdown {
    Device,
    Brand,
    Model,
    Browser,
    BrowserVersion,
    Os,
    OsVersion,
}

#[derive(Debug, Args)]
pub struct OpenpanelGeoArgs {
    #[command(flatten)]
    pub list: OpenpanelBreakdownListArgs,
    #[arg(long, value_enum, default_value_t = OpenpanelGeoBreakdown::Country)]
    pub breakdown: OpenpanelGeoBreakdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OpenpanelGeoBreakdown {
    Country,
    Region,
    City,
}

#[derive(Debug, Args)]
pub struct OpenpanelProfilesCommand {
    #[command(subcommand)]
    pub action: OpenpanelProfilesAction,
}

#[derive(Debug, Subcommand)]
pub enum OpenpanelProfilesAction {
    /// Search and filter user profiles.
    List(OpenpanelProfilesList),
    /// Get a single profile with its most recent events.
    Get(OpenpanelProfileGet),
}

#[derive(Debug, Args)]
pub struct OpenpanelProfilesList {
    /// Project to query. Falls back to profile.project_id.
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub email: Option<String>,
    #[arg(long)]
    pub country: Option<String>,
    #[arg(long)]
    pub city: Option<String>,
    #[arg(long)]
    pub device: Option<String>,
    #[arg(long)]
    pub browser: Option<String>,
    #[arg(long)]
    pub inactive_days: Option<u32>,
    #[arg(long)]
    pub min_sessions: Option<u32>,
    #[arg(long)]
    pub performed_event: Option<String>,
    /// JSON array of chart event filters, passed through to the provider as-is.
    #[arg(long)]
    pub filters: Option<String>,
    #[arg(long, value_enum, default_value_t = OpenpanelSortOrder::Desc)]
    pub sort_order: OpenpanelSortOrder,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OpenpanelSortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Args)]
pub struct OpenpanelProfileGet {
    pub profile_id: String,
    /// Project to query. Falls back to profile.project_id.
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long, default_value_t = 20)]
    pub event_limit: u32,
}

#[derive(Debug, Args)]
pub struct PosthogCommand {
    #[command(subcommand)]
    pub resource: PosthogResource,
}

#[derive(Debug, Subcommand)]
pub enum PosthogResource {
    /// Read PostHog projects.
    Projects(PosthogProjectsCommand),
    /// Execute HogQL or JSON analytics queries against PostHog events.
    Events(PosthogEventsCommand),
    /// List and read saved insights (trends, funnels, retention, paths, lifecycle).
    Insights(PosthogInsightsCommand),
    /// List and inspect persons/users tracked in PostHog.
    Persons(PosthogPersonsCommand),
    /// List and inspect user cohorts.
    Cohorts(PosthogCohortsCommand),
    /// List and read team dashboards.
    Dashboards(PosthogDashboardsCommand),
    /// Read release and experiment annotations.
    Annotations(PosthogAnnotationsCommand),
}

#[derive(Debug, Args)]
pub struct PosthogProjectsCommand {
    #[command(subcommand)]
    pub action: PosthogProjectsAction,
}

#[derive(Debug, Subcommand)]
pub enum PosthogProjectsAction {
    /// List accessible PostHog projects.
    List(PosthogProjectListArgs),
    /// Get details of a specific PostHog project.
    Get(PosthogProjectIdArg),
}

#[derive(Debug, Args)]
pub struct PosthogProjectIdArg {
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct PosthogProjectListArgs {
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
}

#[derive(Debug, Args)]
pub struct PosthogEventsCommand {
    #[command(subcommand)]
    pub action: PosthogEventsAction,
}

#[derive(Debug, Subcommand)]
pub enum PosthogEventsAction {
    /// Execute a query (HogQL query string or JSON payload) against PostHog events.
    Query(PosthogEventsQueryArgs),
}

#[derive(Debug, Args)]
pub struct PosthogEventsQueryArgs {
    #[arg(long)]
    pub project_id: Option<String>,
    /// HogQL query string (e.g. "SELECT event, count() FROM events GROUP BY event LIMIT 10") or JSON query object string.
    #[arg(long)]
    pub query: Option<String>,
    /// Path to a JSON file containing the PostHog query payload object.
    #[arg(long)]
    pub query_file: Option<String>,
}

#[derive(Debug, Args)]
pub struct PosthogInsightsCommand {
    #[command(subcommand)]
    pub action: PosthogInsightsAction,
}

#[derive(Debug, Subcommand)]
pub enum PosthogInsightsAction {
    /// List saved team insights.
    List(PosthogProjectListArgs),
    /// Get details of a specific insight.
    Get(PosthogInsightGetArgs),
}

#[derive(Debug, Args)]
pub struct PosthogInsightGetArgs {
    #[arg(long)]
    pub project_id: Option<String>,
    pub insight_id: String,
}

#[derive(Debug, Args)]
pub struct PosthogPersonsCommand {
    #[command(subcommand)]
    pub action: PosthogPersonsAction,
}

#[derive(Debug, Subcommand)]
pub enum PosthogPersonsAction {
    /// List persons/users tracked in PostHog.
    List(PosthogProjectListArgs),
    /// Get details of a specific person profile.
    Get(PosthogPersonGetArgs),
}

#[derive(Debug, Args)]
pub struct PosthogPersonGetArgs {
    #[arg(long)]
    pub project_id: Option<String>,
    pub person_id: String,
}

#[derive(Debug, Args)]
pub struct PosthogCohortsCommand {
    #[command(subcommand)]
    pub action: PosthogCohortsAction,
}

#[derive(Debug, Subcommand)]
pub enum PosthogCohortsAction {
    /// List user cohorts.
    List(PosthogProjectListArgs),
    /// Get details of a specific cohort.
    Get(PosthogCohortGetArgs),
}

#[derive(Debug, Args)]
pub struct PosthogCohortGetArgs {
    #[arg(long)]
    pub project_id: Option<String>,
    pub cohort_id: String,
}

#[derive(Debug, Args)]
pub struct PosthogDashboardsCommand {
    #[command(subcommand)]
    pub action: PosthogDashboardsAction,
}

#[derive(Debug, Subcommand)]
pub enum PosthogDashboardsAction {
    /// List team dashboards.
    List(PosthogProjectListArgs),
    /// Get details and insight tiles of a specific dashboard.
    Get(PosthogDashboardGetArgs),
}

#[derive(Debug, Args)]
pub struct PosthogDashboardGetArgs {
    #[arg(long)]
    pub project_id: Option<String>,
    pub dashboard_id: String,
}

#[derive(Debug, Args)]
pub struct PosthogAnnotationsCommand {
    #[command(subcommand)]
    pub action: PosthogAnnotationsAction,
}

#[derive(Debug, Subcommand)]
pub enum PosthogAnnotationsAction {
    /// List release and experiment annotations.
    List(PosthogProjectListArgs),
    /// Get details of a specific annotation.
    Get(PosthogAnnotationGetArgs),
}

#[derive(Debug, Args)]
pub struct PosthogAnnotationGetArgs {
    #[arg(long)]
    pub project_id: Option<String>,
    pub annotation_id: String,
}
