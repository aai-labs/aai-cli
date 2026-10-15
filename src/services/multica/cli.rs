use super::{
    endpoint::{Endpoint, Paging},
    registry,
};
use crate::cli::GenericRequest;
use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct MulticaCommand {
    pub workspace: Option<String>,
    pub(super) invocation: Invocation,
}

#[derive(Debug)]
pub(super) enum Invocation {
    Request(GenericRequest),
    Endpoint(Endpoint, Box<ArgMatches>),
}

impl Args for MulticaCommand {
    fn augment_args(command: Command) -> Command {
        tree(command, &registry::endpoints(), 0)
            .arg(
                Arg::new("workspace")
                    .long("workspace")
                    .global(true)
                    .help("Workspace UUID or slug; overrides profile.workspace"),
            )
            .subcommand(GenericRequest::augment_args(Command::new("request").about(
                "Call a relative API endpoint; writes require --allow-write",
            )))
    }
    fn augment_args_for_update(command: Command) -> Command {
        Self::augment_args(command)
    }
}

impl FromArgMatches for MulticaCommand {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let workspace = matches.get_one::<String>("workspace").cloned();
        if let Some(("request", args)) = matches.subcommand() {
            return Ok(Self {
                workspace,
                invocation: Invocation::Request(GenericRequest::from_arg_matches(args)?),
            });
        }
        let mut parts = Vec::new();
        let mut leaf = matches;
        while let Some((name, args)) = leaf.subcommand() {
            parts.push(name);
            leaf = args;
        }
        let name = parts.join(" ");
        let endpoint = registry::endpoints()
            .into_iter()
            .find(|e| e.command == name)
            .ok_or_else(|| {
                clap::Error::raw(
                    clap::error::ErrorKind::MissingSubcommand,
                    "select a Multica resource and action",
                )
            })?;
        Ok(Self {
            workspace,
            invocation: Invocation::Endpoint(endpoint, Box::new(leaf.clone())),
        })
    }
    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

fn tree(mut command: Command, endpoints: &[Endpoint], depth: usize) -> Command {
    let names: BTreeSet<_> = endpoints
        .iter()
        .filter_map(|e| e.command.split_whitespace().nth(depth))
        .collect();
    for name in names {
        let subset: Vec<_> = endpoints
            .iter()
            .filter(|e| e.command.split_whitespace().nth(depth) == Some(name))
            .cloned()
            .collect();
        let child = Command::new(name.to_owned());
        let child = if let Some(e) = subset
            .iter()
            .find(|e| e.command.split_whitespace().count() == depth + 1)
        {
            leaf(child, e)
        } else {
            tree(child, &subset, depth + 1)
        };
        command = command.subcommand(child);
    }
    command
        .subcommand_required(true)
        .arg_required_else_help(true)
}

fn option(key: &str) -> Arg {
    let mut arg = Arg::new(key.to_owned())
        .long(key.replace('_', "-"))
        .help(format!("Multica `{key}` field"));
    if boolean(key) {
        arg = arg
            .num_args(0..=1)
            .default_missing_value("true")
            .value_parser(["true", "false"]);
    }
    if key == "member_type" {
        arg = arg.value_parser(["member", "agent"]);
    }
    if key == "assignee_type" {
        arg = arg.value_parser(["member", "agent", "squad"]);
    }
    arg
}

fn leaf(mut command: Command, e: &Endpoint) -> Command {
    command = command.about(format!("{} /api{}", e.method, e.path));
    let mut keys = BTreeSet::new();
    for key in e.path_fields() {
        keys.insert(key.to_owned());
        command = command.arg(option(key).required(true));
    }
    for key in e
        .query
        .split_whitespace()
        .chain(e.fields.split_whitespace())
    {
        if keys.insert(key.to_owned()) {
            command = command.arg(option(key));
        }
    }
    if e.method == "GET" {
        command = command.arg(Arg::new("query").long("query").action(ArgAction::Append)
            .help("Additional provider query parameter, KEY=VALUE (repeatable); typed flags take precedence"));
    } else {
        command = command.arg(
            Arg::new("json")
                .long("json")
                .help("JSON object from file or - (stdin); typed flags override fields"),
        );
    }
    if e.paging != Paging::None {
        command = command.arg(Arg::new("limit").long("limit").value_parser(clap::value_parser!(usize)).default_value("100")
            .help("Maximum total records to return; 0 makes no request"))
            .arg(Arg::new("page_skip").long("page-skip").value_parser(clap::value_parser!(usize)).default_value("0")
            .help("Resume within a provider page using _aai.pagination continuation; not a server cursor"));
    }
    if matches!(e.paging, Paging::Offset | Paging::OffsetUntilShort) && keys.insert("offset".into())
    {
        command = command
            .arg(option("offset").value_parser(clap::builder::NonEmptyStringValueParser::new()));
    }
    command
}

pub(super) fn boolean(key: &str) -> bool {
    matches!(
        key,
        "disabled"
            | "apply_to_machine"
            | "open_only"
            | "enabled"
            | "include_closed"
            | "include_archived"
            | "include_usage"
            | "unread_only"
            | "roots_only"
            | "summary"
            | "fold"
            | "active"
            | "scheduled"
            | "top_level_only"
            | "include_no_assignee"
            | "include_no_project"
    )
}
