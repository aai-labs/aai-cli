use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Value};
use zip::ZipArchive;

use crate::{
    cli::{SkillsAction, SkillsCommand, SkillsInstall, SkillsValidate},
    error::AppError,
};

const BUNDLED_SKILLS_ZIP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/aai-skills.zip"));
const SERVICE: &str = "skills";

const SKILL_COMMANDS: &[(&str, &[&str])] = &[
    ("aai-apollo", &["apollo"]),
    ("aai-bitbucket", &["bitbucket"]),
    ("aai-confluence", &["confluence"]),
    ("aai-excel", &["excel"]),
    ("aai-github", &["github"]),
    ("aai-hubspot", &["hubspot"]),
    ("aai-gmail", &["email"]),
    ("aai-google-drive", &["drive"]),
    ("aai-google-sheets", &["sheets"]),
    ("aai-jira", &["jira"]),
    ("aai-microsoft", &["microsoft"]),
    ("aai-pipedrive", &["pipedrive"]),
    ("aai-posthog", &["posthog"]),
    ("aai-slack", &["slack"]),
    ("aai-zoho-mail", &["email"]),
];

const TOP_LEVEL_COMMANDS: &[&str] = &[
    "jira",
    "confluence",
    "bitbucket",
    "github",
    "hubspot",
    "email",
    "calendar",
    "pipedrive",
    "apollo",
    "sheets",
    "drive",
    "excel",
    "slack",
    "posthog",
    "microsoft",
    "config",
    "secrets",
    "skills",
];

#[derive(Debug)]
struct SkillPackage {
    name: String,
    skill_md: String,
    files: BTreeSet<String>,
}

#[derive(Debug, Serialize)]
struct ValidationReport {
    name: String,
    valid: bool,
    errors: Vec<String>,
}

#[derive(Debug)]
struct Frontmatter {
    name: String,
    description: String,
}

pub(crate) fn dispatch(command: SkillsCommand) -> Result<Value, AppError> {
    match command.action {
        SkillsAction::Discover => discover(),
        SkillsAction::Install(args) => install(args),
        SkillsAction::Validate(args) => validate(args),
    }
}

fn discover() -> Result<Value, AppError> {
    let packages = load_packages()?;
    let reports = validate_packages(&packages);

    let mut report_by_name = BTreeMap::new();
    for report in reports {
        report_by_name.insert(report.name.clone(), report);
    }

    let skills = packages
        .iter()
        .map(|package| {
            let frontmatter = parse_frontmatter(&package.skill_md).ok();
            let commands = commands_for_skill(&package.name);
            let report = report_by_name
                .get(&package.name)
                .expect("validation report exists for every package");
            json!({
                "name": package.name,
                "description": frontmatter.map(|value| value.description).unwrap_or_default(),
                "commands": commands,
                "valid": report.valid,
                "errors": report.errors,
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "skills": skills,
        "command_coverage": command_coverage(),
    }))
}

fn validate(args: SkillsValidate) -> Result<Value, AppError> {
    let packages = select_packages(args.skill_name.as_deref())?;
    let reports = validate_packages(&packages);
    let valid = reports.iter().all(|report| report.valid);
    Ok(json!({
        "valid": valid,
        "skills": reports,
    }))
}

fn install(args: SkillsInstall) -> Result<Value, AppError> {
    if !args.all && args.skill_name.is_none() {
        return Err(AppError::invalid_input(
            SERVICE,
            "install",
            "provide a skill name or --all",
        ));
    }

    let packages = if args.all {
        load_packages()?
    } else {
        select_packages(args.skill_name.as_deref())?
    };

    let reports = validate_packages(&packages);
    if let Some(report) = reports.iter().find(|report| !report.valid) {
        return Err(AppError::invalid_input(
            SERVICE,
            "install",
            format!(
                "bundled skill package '{}' is invalid: {}",
                report.name,
                report.errors.join("; ")
            ),
        ));
    }

    let target_dir = args.target_dir.unwrap_or(default_target_dir()?);
    let mut installed = Vec::new();

    for package in packages {
        let destination = target_dir.join(&package.name);
        let exists = destination.exists();
        if exists && !args.force {
            return Err(AppError::invalid_input(
                SERVICE,
                "install",
                format!(
                    "skill '{}' already exists at {}; pass --force to overwrite",
                    package.name,
                    destination.display()
                ),
            ));
        }

        let action = match (args.dry_run, exists) {
            (true, true) => "would_overwrite",
            (true, false) => "would_install",
            (false, true) => "overwritten",
            (false, false) => "installed",
        };

        if !args.dry_run {
            if exists {
                fs::remove_dir_all(&destination).map_err(|err| {
                    AppError::internal(SERVICE, "install", format!("remove existing skill: {err}"))
                })?;
            }
            extract_package(&package.name, &target_dir)?;
        }

        installed.push(json!({
            "name": package.name,
            "path": destination,
            "action": action,
        }));
    }

    Ok(json!({
        "target_dir": target_dir,
        "dry_run": args.dry_run,
        "installed": installed,
    }))
}

fn load_packages() -> Result<Vec<SkillPackage>, AppError> {
    let mut archive = open_archive()?;
    let mut packages = BTreeMap::<String, SkillPackage>::new();

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(zip_error("load"))?;
        if !file.is_file() {
            continue;
        }
        let name = normalize_archive_name(file.name())?;
        let Some((package_name, _)) = name.split_once('/') else {
            continue;
        };

        let package = packages
            .entry(package_name.to_string())
            .or_insert_with(|| SkillPackage {
                name: package_name.to_string(),
                skill_md: String::new(),
                files: BTreeSet::new(),
            });

        package.files.insert(name.clone());
        if name.ends_with("/SKILL.md") {
            let mut contents = String::new();
            file.read_to_string(&mut contents).map_err(|err| {
                AppError::internal(SERVICE, "load", format!("read {name}: {err}"))
            })?;
            package.skill_md = contents;
        }
    }

    Ok(packages.into_values().collect())
}

fn select_packages(skill_name: Option<&str>) -> Result<Vec<SkillPackage>, AppError> {
    let packages = load_packages()?;
    if let Some(skill_name) = skill_name {
        let package = packages
            .into_iter()
            .find(|package| package.name == skill_name)
            .ok_or_else(|| {
                AppError::invalid_input(
                    SERVICE,
                    "select",
                    format!("unknown bundled skill '{skill_name}'"),
                )
            })?;
        Ok(vec![package])
    } else {
        Ok(packages)
    }
}

fn validate_packages(packages: &[SkillPackage]) -> Vec<ValidationReport> {
    packages.iter().map(validate_package).collect()
}

fn validate_package(package: &SkillPackage) -> ValidationReport {
    let mut errors = Vec::new();

    if !is_valid_package_name(&package.name) {
        errors
            .push("package directory name must use lowercase letters, numbers, and hyphens".into());
    }

    if package.skill_md.trim().is_empty() {
        errors.push("missing SKILL.md".into());
    } else {
        match parse_frontmatter(&package.skill_md) {
            Ok(frontmatter) => {
                if frontmatter.name != package.name {
                    errors.push(format!(
                        "frontmatter name '{}' does not match package '{}'",
                        frontmatter.name, package.name
                    ));
                }
                if frontmatter.description.trim().is_empty() {
                    errors.push("frontmatter description is empty".into());
                }
            }
            Err(err) => errors.push(err),
        }

        errors.extend(validate_reference_links(package));
    }

    ValidationReport {
        name: package.name.clone(),
        valid: errors.is_empty(),
        errors,
    }
}

fn parse_frontmatter(contents: &str) -> Result<Frontmatter, String> {
    let mut lines = contents.lines();
    if lines.next() != Some("---") {
        return Err("SKILL.md must start with YAML frontmatter".into());
    }

    let mut values = BTreeMap::new();
    for line in lines.by_ref() {
        if line == "---" {
            let name = values
                .remove("name")
                .ok_or_else(|| "frontmatter missing name".to_string())?;
            let description = values
                .remove("description")
                .ok_or_else(|| "frontmatter missing description".to_string())?;
            return Ok(Frontmatter { name, description });
        }

        if let Some((key, value)) = line.split_once(':') {
            values.insert(key.trim().to_string(), trim_yaml_scalar(value));
        }
    }

    Err("SKILL.md frontmatter is not closed".into())
}

fn trim_yaml_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn validate_reference_links(package: &SkillPackage) -> Vec<String> {
    let mut errors = Vec::new();
    for link in markdown_links(&package.skill_md) {
        if is_external_or_anchor(&link) {
            continue;
        }

        let path_without_anchor = link.split('#').next().unwrap_or_default();
        if path_without_anchor.is_empty() {
            continue;
        }

        let normalized = format!("{}/{}", package.name, path_without_anchor);
        if !package.files.contains(&normalized) {
            errors.push(format!("broken local reference '{link}'"));
        }
    }
    errors
}

fn markdown_links(contents: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = contents;
    while let Some(close_bracket) = rest.find("](") {
        rest = &rest[close_bracket + 2..];
        let Some(close_paren) = rest.find(')') else {
            break;
        };
        links.push(rest[..close_paren].to_string());
        rest = &rest[close_paren + 1..];
    }
    links
}

fn is_external_or_anchor(link: &str) -> bool {
    link.starts_with('#')
        || link.starts_with("http://")
        || link.starts_with("https://")
        || link.starts_with("mailto:")
}

fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

fn command_coverage() -> Vec<Value> {
    TOP_LEVEL_COMMANDS
        .iter()
        .map(|command| {
            let skills = SKILL_COMMANDS
                .iter()
                .filter_map(|(skill, commands)| {
                    if commands.contains(command) {
                        Some(*skill)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            json!({
                "command": command,
                "skills": skills,
                "covered": !skills.is_empty(),
            })
        })
        .collect()
}

fn commands_for_skill(skill_name: &str) -> Vec<&'static str> {
    SKILL_COMMANDS
        .iter()
        .find_map(|(skill, commands)| {
            if *skill == skill_name {
                Some(commands.to_vec())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn extract_package(package_name: &str, target_dir: &Path) -> Result<(), AppError> {
    let mut archive = open_archive()?;
    let package_prefix = format!("{package_name}/");

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(zip_error("extract"))?;
        if !file.is_file() || !file.name().starts_with(&package_prefix) {
            continue;
        }

        let archive_name = normalize_archive_name(file.name())?;
        let relative = archive_name
            .strip_prefix(&package_prefix)
            .expect("file matched package prefix");
        let output_path = checked_join(target_dir, &archive_name)?;
        if relative.is_empty() {
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                AppError::internal(
                    SERVICE,
                    "install",
                    format!("create {}: {err}", parent.display()),
                )
            })?;
        }
        let mut output = fs::File::create(&output_path).map_err(|err| {
            AppError::internal(
                SERVICE,
                "install",
                format!("create {}: {err}", output_path.display()),
            )
        })?;
        std::io::copy(&mut file, &mut output).map_err(|err| {
            AppError::internal(
                SERVICE,
                "install",
                format!("write {}: {err}", output_path.display()),
            )
        })?;
    }

    Ok(())
}

fn checked_join(base: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(AppError::internal(
            SERVICE,
            "extract",
            format!("archive contains unsafe path '{relative}'"),
        ));
    }
    Ok(base.join(path))
}

fn normalize_archive_name(name: &str) -> Result<String, AppError> {
    if name.contains('\\') {
        return Err(AppError::internal(
            SERVICE,
            "load",
            format!("archive path contains backslash: {name}"),
        ));
    }
    checked_join(Path::new(""), name)?;
    Ok(name.to_string())
}

fn open_archive() -> Result<ZipArchive<Cursor<&'static [u8]>>, AppError> {
    ZipArchive::new(Cursor::new(BUNDLED_SKILLS_ZIP)).map_err(zip_error("load"))
}

fn zip_error(operation: &'static str) -> impl Fn(zip::result::ZipError) -> AppError {
    move |err| AppError::internal(SERVICE, operation, err.to_string())
}

fn default_target_dir() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir().ok_or_else(|| {
        AppError::config("could not determine home directory for default skills target")
    })?;
    Ok(home.join(".agents").join("skills"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_parser_reads_name_and_description() {
        let frontmatter = parse_frontmatter(
            "---\nname: aai-example\ndescription: Example skill package.\n---\n# Body\n",
        )
        .expect("frontmatter parses");

        assert_eq!(frontmatter.name, "aai-example");
        assert_eq!(frontmatter.description, "Example skill package.");
    }

    #[test]
    fn bundled_skills_validate() {
        let packages = load_packages().expect("load bundled skills");
        let reports = validate_packages(&packages);

        assert_eq!(reports.len(), 16);
        assert!(
            reports.iter().all(|report| report.valid),
            "invalid reports: {reports:#?}"
        );
    }

    #[test]
    fn validation_catches_broken_local_reference() {
        let package = SkillPackage {
            name: "aai-example".into(),
            skill_md: "---\nname: aai-example\ndescription: Example.\n---\n[missing](references/nope.md)\n"
                .into(),
            files: BTreeSet::from(["aai-example/SKILL.md".into()]),
        };

        let report = validate_package(&package);

        assert!(!report.valid);
        assert_eq!(
            report.errors,
            vec!["broken local reference 'references/nope.md'"]
        );
    }

    #[test]
    fn discover_reports_uncovered_commands() {
        let coverage = command_coverage();
        let calendar = coverage
            .iter()
            .find(|entry| entry["command"] == "calendar")
            .expect("calendar coverage entry");

        assert_eq!(calendar["covered"], false);
        assert_eq!(calendar["skills"], json!([]));
    }

    #[test]
    fn discover_reports_microsoft_skill_coverage() {
        let coverage = command_coverage();
        let microsoft = coverage
            .iter()
            .find(|entry| entry["command"] == "microsoft")
            .expect("microsoft coverage entry");

        assert_eq!(microsoft["covered"], true);
        assert_eq!(microsoft["skills"], json!(["aai-microsoft"]));
    }

    #[test]
    fn dry_run_install_does_not_write_files() {
        let temp = tempfile::tempdir().expect("tempdir");

        let value = install(SkillsInstall {
            skill_name: Some("aai-github".into()),
            all: false,
            target_dir: Some(temp.path().into()),
            force: false,
            dry_run: true,
        })
        .expect("dry-run install succeeds");

        assert_eq!(value["dry_run"], true);
        assert!(!temp.path().join("aai-github").exists());
    }

    #[test]
    fn install_refuses_to_overwrite_without_force() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("aai-github")).expect("existing skill dir");

        let err = install(SkillsInstall {
            skill_name: Some("aai-github".into()),
            all: false,
            target_dir: Some(temp.path().into()),
            force: false,
            dry_run: false,
        })
        .expect_err("install refuses overwrite");

        assert_eq!(err.code, "invalid_input");
        assert!(err.message.contains("already exists"));
    }

    #[test]
    fn install_with_force_extracts_skill() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("aai-github")).expect("existing skill dir");

        install(SkillsInstall {
            skill_name: Some("aai-github".into()),
            all: false,
            target_dir: Some(temp.path().into()),
            force: true,
            dry_run: false,
        })
        .expect("forced install succeeds");

        assert!(temp.path().join("aai-github/SKILL.md").exists());
        assert!(temp
            .path()
            .join("aai-github/references/command-reference.md")
            .exists());
    }
}
