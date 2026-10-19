use super::{registry, Endpoint, Field};
use crate::cli::GenericRequest;
use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct AgentBarnCommand {
    pub(super) organization: Option<String>,
    pub(super) invocation: Invocation,
}
#[derive(Debug)]
pub(super) enum Invocation {
    Request(GenericRequest),
    Endpoint(&'static Endpoint, Box<ArgMatches>),
}
impl Args for AgentBarnCommand {
    fn augment_args(command: Command) -> Command {
        tree(command, &registry::endpoints().iter().collect::<Vec<_>>(), 0)
            .arg(Arg::new("organization_id").long("organization-id").global(true)
                .help("Organization UUID; overrides profile.org (also a platform filter where supported)"))
            .subcommand(GenericRequest::augment_args(Command::new("request").about("Call an API-relative path; writes require --allow-write; single response")))
    }
    fn augment_args_for_update(command: Command) -> Command {
        Self::augment_args(command)
    }
}
impl FromArgMatches for AgentBarnCommand {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let organization = matches.get_one::<String>("organization_id").cloned();
        let invocation = if let Some(("request", args)) = matches.subcommand() {
            Invocation::Request(GenericRequest::from_arg_matches(args)?)
        } else {
            let mut leaf = matches;
            let mut parts = Vec::new();
            while let Some((name, args)) = leaf.subcommand() {
                parts.push(name);
                leaf = args;
            }
            let name = parts.join(" ");
            let endpoint = registry::endpoints()
                .iter()
                .find(|e| e.command == name)
                .ok_or_else(|| {
                    clap::Error::raw(
                        clap::error::ErrorKind::MissingSubcommand,
                        "select an Agent Barn resource and action",
                    )
                })?;
            Invocation::Endpoint(endpoint, Box::new(leaf.clone()))
        };
        Ok(Self {
            organization,
            invocation,
        })
    }
    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}
fn tree(mut command: Command, endpoints: &[&Endpoint], depth: usize) -> Command {
    let names: BTreeSet<_> = endpoints
        .iter()
        .filter_map(|e| e.command.split_whitespace().nth(depth))
        .collect();
    for name in names {
        let subset: Vec<_> = endpoints
            .iter()
            .copied()
            .filter(|e| e.command.split_whitespace().nth(depth) == Some(name))
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
fn option(field: &Field) -> Arg {
    let mut arg = Arg::new(field.name.clone()).long(field.name.replace('_', "-"));
    if field.kind == "boolean" {
        arg = arg
            .num_args(0..=1)
            .default_missing_value("true")
            .value_parser(["true", "false"]);
    } else if !field.choices.is_empty() {
        arg = arg.value_parser(clap::builder::PossibleValuesParser::new(
            field.choices.clone(),
        ));
    } else {
        arg = arg.value_parser(clap::builder::NonEmptyStringValueParser::new());
    }
    if field.kind == "array" {
        arg = arg.action(ArgAction::Append);
    }
    arg
}
fn leaf(mut command: Command, e: &Endpoint) -> Command {
    command = command.about(format!(
        "{} {}{}",
        e.method,
        e.path,
        if e.deprecated {
            " (deprecated upstream)"
        } else {
            ""
        }
    ));
    command = command.long_about(format!(
        "{} {}\nOperation: {}{}",
        e.method,
        e.path,
        e.operation_id,
        if e.deprecated {
            " (deprecated upstream)"
        } else {
            ""
        }
    ));
    let mut keys = BTreeSet::from(["organization_id".to_owned()]);
    for f in &e.path_fields {
        if keys.insert(f.name.clone()) {
            command = command.arg(option(f).required(true));
        }
    }
    for f in &e.query {
        if f.name == "limit" && e.paging != "none" {
            continue;
        }
        if keys.insert(f.name.clone()) {
            command = command.arg(option(f).required(f.required));
        }
    }
    for f in &e.fields {
        if keys.insert(f.name.clone()) {
            command = command.arg(option(f).help("Body field; overrides --json"));
        }
    }
    if e.method != "GET" {
        command = command.arg(
            Arg::new("json")
                .long("json")
                .help("JSON body file or - for stdin; scalar flags override fields"),
        );
    }
    if e.paging != "none" {
        command = command
            .arg(
                Arg::new("limit")
                    .long("limit")
                    .default_value("100")
                    .value_parser(clap::value_parser!(usize)),
            )
            .arg(
                Arg::new("page_skip")
                    .long("page-skip")
                    .default_value("0")
                    .value_parser(clap::value_parser!(usize))
                    .help("Resume within a provider page; use the returned next_command"),
            );
    }
    if e.mode != "json" {
        command = command.arg(
            Arg::new("output")
                .long("output")
                .required(true)
                .help("New output file; existing files are never overwritten"),
        );
    }
    if e.mode == "stream" {
        command = command.arg(
            Arg::new("duration_seconds")
                .long("duration-seconds")
                .default_value("30")
                .value_parser(clap::value_parser!(u64).range(1..=86400)),
        );
    }
    command
}
