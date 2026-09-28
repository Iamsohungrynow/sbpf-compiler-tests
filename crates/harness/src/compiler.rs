use std::{
    error::Error,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{cargo, command_error};

pub(crate) const CARGO_BUILD_SBPF: &str = "cargo-build-sbpf";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compiler {
    CargoBuildSbpf,
}

impl Compiler {
    pub(crate) fn subcommand(self) -> &'static str {
        match self {
            Self::CargoBuildSbpf => "build-sbpf",
        }
    }

    pub(crate) fn build_args(self) -> &'static [&'static str] {
        match self {
            Self::CargoBuildSbpf => &[],
        }
    }

    pub(crate) fn target_dir(self) -> &'static str {
        match self {
            Self::CargoBuildSbpf => "target/bpfel-unknown-none/release",
        }
    }

    pub(crate) fn elf_path(self, root: &Path, target_name: &str) -> PathBuf {
        let file_name = match self {
            Self::CargoBuildSbpf => format!("lib{target_name}.so"),
        };
        root.join(self.target_dir()).join(file_name)
    }

    pub(crate) fn tool_name(self) -> &'static str {
        match self {
            Self::CargoBuildSbpf => CARGO_BUILD_SBPF,
        }
    }

    pub(crate) fn version(self) -> Result<Vec<(String, String)>, Box<dyn Error>> {
        let cargo = cargo().to_string_lossy().into_owned();
        match self {
            Self::CargoBuildSbpf => {
                let mut versions = run_version_command(&cargo, &[self.subcommand(), "--version"])?;
                versions.extend(run_version_command("sbpf-linker", &["--version"])?);
                versions.extend(run_version_command("rustc", &["--version"])?);
                Ok(versions)
            }
        }
    }
}

fn run_version_command(
    command: &str,
    args: &[&str],
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let description = format!("{} {}", command, args.join(" "));
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        return Err(command_error(&description, &output));
    }
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.split_once(' ') {
            Some((label, version)) => (label.to_owned(), version.to_owned()),
            None => (line.to_owned(), String::new()),
        })
        .collect())
}
