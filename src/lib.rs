//! Starts `sqlakit-lsp` for the SQL templates and the Python of a project.
//!
//! The server is a Python package, found in the project's environment, or
//! downloaded by `uvx` when the project has none:
//!
//! 1. `lsp.sqlakit-lsp.binary.path` in Zed's settings, with its arguments and
//!    environment;
//! 2. `.venv/bin/sqlakit-lsp` in the project;
//! 3. `sqlakit-lsp` on the `PATH` of the project's shell;
//! 4. `uvx sqlakit-lsp`, which installs it on first use, with the `sqlakit`
//!    version the project's `uv.lock` holds.
//!
//! Zed asks for the server in every Python project, so a project that does not
//! depend on `sqlakit` gets none, unless the settings name one.

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

const SERVER: &str = "sqlakit-lsp";

/// The oldest `sqlakit` the server reads the templates of.
const OLDEST: (u32, u32) = (0, 21);

/// The files that say a project depends on `sqlakit`, read in this order.
const DEPENDENCIES: [&str; 4] = [
    "pyproject.toml",
    "uv.lock",
    "requirements.txt",
    "requirements-dev.txt",
];

struct SqlakitExtension;

impl SqlakitExtension {
    /// The server in the project's virtual environment, if it has one there.
    fn in_venv(worktree: &zed::Worktree) -> Option<String> {
        let (os, _) = zed::current_platform();
        let relative = match os {
            zed::Os::Windows => ".venv/Scripts/sqlakit-lsp.exe",
            _ => ".venv/bin/sqlakit-lsp",
        };
        // A file the worktree can read is there; its text is not needed.
        worktree.read_text_file(relative).ok()?;
        Some(format!("{}/{relative}", worktree.root_path()))
    }

    /// Whether the project names `sqlakit` in any file of its dependencies.
    fn uses_sqlakit(worktree: &zed::Worktree) -> bool {
        DEPENDENCIES.iter().any(|file| {
            worktree
                .read_text_file(file)
                .is_ok_and(|text| names_sqlakit(&text))
        })
    }
}

impl zed::Extension for SqlakitExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary = LspSettings::for_worktree(SERVER, worktree)
            .ok()
            .and_then(|settings| settings.binary);
        let mut env = worktree.shell_env();
        if let Some(extra) = binary.as_ref().and_then(|binary| binary.env.clone()) {
            env.extend(extra);
        }
        let args = binary
            .as_ref()
            .and_then(|binary| binary.arguments.clone())
            .unwrap_or_default();
        // A server the settings name runs wherever they say.
        if let Some(command) = binary.and_then(|binary| binary.path) {
            return Ok(zed::Command { command, args, env });
        }
        if !Self::uses_sqlakit(worktree) {
            return Err(format!(
                "the project does not depend on sqlakit, so {SERVER} is not started"
            ));
        }
        let found = Self::in_venv(worktree).or_else(|| worktree.which(SERVER));
        if let Some(command) = found {
            return Ok(zed::Command { command, args, env });
        }
        let uvx = worktree.which("uvx").ok_or_else(|| {
            format!(
                "{SERVER} is not installed: `pip install {SERVER}` in the project's \
                 environment, install uv for `uvx {SERVER}`, or set \
                 lsp.{SERVER}.binary.path in Zed's settings"
            )
        })?;
        let lock = worktree.read_text_file("uv.lock").unwrap_or_default();
        let locked = locked_version(&lock, "sqlakit");
        if let Some(version) = &locked {
            if !reads(version) {
                return Err(format!(
                    "{SERVER} reads the templates of sqlakit {}.{} and newer, and \
                     uv.lock holds sqlakit {version}",
                    OLDEST.0, OLDEST.1
                ));
            }
        }
        let mut uvx_args = Vec::new();
        if let Some(version) = locked {
            uvx_args.extend(["--with".to_string(), format!("sqlakit=={version}")]);
        }
        uvx_args.push(SERVER.to_string());
        uvx_args.extend(args);
        Ok(zed::Command {
            command: uvx,
            args: uvx_args,
            env,
        })
    }
}

/// Whether a file of dependencies names `sqlakit` itself, and not only a
/// package whose name holds it.
fn names_sqlakit(text: &str) -> bool {
    let part_of_a_name = |c: char| c == '-' || c == '_' || c.is_alphanumeric();
    text.match_indices("sqlakit").any(|(start, found)| {
        let before = text[..start].chars().next_back();
        let after = text[start + found.len()..].chars().next();
        !before.is_some_and(part_of_a_name) && !after.is_some_and(part_of_a_name)
    })
}

/// Whether the server reads the templates of this `sqlakit` version.
fn reads(version: &str) -> bool {
    let mut parts = version.split('.').map(|part| part.parse::<u32>().ok());
    match (parts.next().flatten(), parts.next().flatten()) {
        (Some(major), Some(minor)) => (major, minor) >= OLDEST,
        // A version it cannot read is left to uv, which says what is wrong.
        _ => true,
    }
}

/// The version of a package a `uv.lock` holds, so the server reads the
/// templates with the `sqlakit` the project runs.
fn locked_version(lock: &str, package: &str) -> Option<String> {
    let name = format!("name = \"{package}\"");
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == name {
            let version = lines.next()?.trim().strip_prefix("version = \"")?;
            return version.strip_suffix('"').map(String::from);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{locked_version, names_sqlakit, reads};

    const LOCK: &str = r#"version = 1

[[package]]
name = "sqlakit"
version = "0.21.0"
source = { registry = "https://pypi.org/simple" }

[[package]]
name = "sqlakit-lsp"
version = "0.1.0"
"#;

    #[test]
    fn the_locked_version_is_the_one_of_that_package() {
        assert_eq!(locked_version(LOCK, "sqlakit").as_deref(), Some("0.21.0"));
        assert_eq!(
            locked_version(LOCK, "sqlakit-lsp").as_deref(),
            Some("0.1.0")
        );
    }

    #[test]
    fn a_package_the_lock_does_not_hold_has_no_version() {
        assert_eq!(locked_version(LOCK, "flask"), None);
        assert_eq!(locked_version("", "sqlakit"), None);
    }

    #[test]
    fn a_project_names_sqlakit_as_a_dependency() {
        assert!(names_sqlakit("dependencies = [\"sqlakit>=0.21\"]"));
        assert!(names_sqlakit("dependencies = [\"sqlakit[asyncio]\"]"));
        assert!(names_sqlakit("sqlakit==0.21.0\n"));
        assert!(names_sqlakit(LOCK));
    }

    #[test]
    fn a_package_that_only_holds_the_name_is_not_it() {
        assert!(!names_sqlakit("dependencies = [\"sqlakit-debugserver\"]"));
        assert!(!names_sqlakit("dependencies = [\"mysqlakitten\"]"));
        assert!(!names_sqlakit("dependencies = [\"flask\"]"));
    }

    #[test]
    fn the_server_reads_sqlakit_from_0_21() {
        assert!(reads("0.21.0"));
        assert!(reads("0.22.3"));
        assert!(reads("1.0.0"));
        assert!(!reads("0.20.0"));
        assert!(reads("not a version"));
    }
}

zed::register_extension!(SqlakitExtension);
