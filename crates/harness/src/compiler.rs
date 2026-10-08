use std::{
    error::Error,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{cargo, command_error};

pub(crate) const CARGO_BUILD_SBPF: &str = "cargo-build-sbpf";
pub(crate) const CARGO_BUILD_SBF: &str = "cargo-build-sbf";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compiler {
    CargoBuildSbpf,
    CargoBuildSbf,
}

impl Compiler {
    pub(crate) fn subcommand(self) -> &'static str {
        match self {
            Self::CargoBuildSbpf => "build-sbpf",
            Self::CargoBuildSbf => "build-sbf",
        }
    }

    pub(crate) fn build_args(self) -> &'static [&'static str] {
        match self {
            Self::CargoBuildSbpf => &[],
            Self::CargoBuildSbf => &["--arch", "v3"], // build with arch v3
        }
    }

    pub(crate) fn target_dir(self) -> &'static str {
        match self {
            Self::CargoBuildSbpf => "target/bpfel-unknown-none/release",
            Self::CargoBuildSbf => "target/sbpfv3-solana-solana/release",
        }
    }

    pub(crate) fn elf_path(self, root: &Path, target_name: &str) -> PathBuf {
        let file_name = match self {
            Self::CargoBuildSbpf => format!("lib{target_name}.so"),
            Self::CargoBuildSbf => format!("{target_name}.so"),
        };
        root.join(self.target_dir()).join(file_name)
    }

    pub(crate) fn tool_name(self) -> &'static str {
        match self {
            Self::CargoBuildSbpf => CARGO_BUILD_SBPF,
            Self::CargoBuildSbf => CARGO_BUILD_SBF,
        }
    }

    pub(crate) fn version(self, binary: &Path) -> Result<Vec<(String, String)>, Box<dyn Error>> {
        match self {
            Self::CargoBuildSbpf => {
                let mut versions = run_version_command(binary, &[self.subcommand(), "--version"])?;
                versions.extend(run_version_command(
                    Path::new("sbpf-linker"),
                    &["--version"],
                )?);
                versions.extend(run_version_command(Path::new("rustc"), &["--version"])?);
                Ok(versions)
            }
            Self::CargoBuildSbf => run_version_command(binary, &[self.subcommand(), "--version"]),
        }
    }
}

fn run_version_command(
    command: &Path,
    args: &[&str],
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let description = format!("{} {}", command.to_string_lossy(), args.join(" "));
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        return Err(command_error(&description, &output));
    }
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(2)
        .map(|line| match line.split_once(' ') {
            Some((label, version)) => (label.to_owned(), version.to_owned()),
            None => (line.to_owned(), String::new()),
        })
        .collect())
}

pub fn build_local_compiler(compiler: Compiler) -> Result<PathBuf, Box<dyn Error>> {
    let status = Command::new(cargo())
        .args(["build", "-p", compiler.tool_name()])
        .status()
        .map_err(|error| format!("failed to run `cargo build`: {error}"))?;
    if !status.success() {
        return Err(format!(
            "failed to build `{}` locally, make sure the current directory is its root",
            compiler.tool_name()
        )
        .into());
    }

    let binary = Path::new("target/debug").join(compiler.tool_name());
    Ok(binary.canonicalize()?)
}
