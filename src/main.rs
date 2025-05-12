use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::Write;
use std::path::PathBuf;
use std::{fmt, fs, io};
use tempfile::NamedTempFile;
extern crate pretty_env_logger;
use std::process::ExitCode;

mod config;
mod git;
mod parse;
mod util;

#[derive(Debug)]
enum Error {
    Exit(ExitCode),
    FileNotFound(PathBuf),
    /// Configuration file could not be parsed.
    InvalidConfiguration(serde_yaml::Error),
    Io(io::Error),
    /// No git branch could be found.
    NoBranch,
    /// No bureucrat configuration could be found.
    NoConfigurationFile,
    /// No git repository could be found.
    NoRepository,
    /// Unknown error during git operations.
    UnknownGit(git2::Error),
}

#[derive(Parser)]
#[command(name = "bureaucrat", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Set logging level.
    #[arg(short, long, env = "BUREAUCRAT_LOG", default_value_t = log::LevelFilter::Info)]
    log_level: log::LevelFilter,
}

#[derive(Subcommand)]
enum Commands {
    /// Install the prepare-commit-msg hook.
    Install(InstallArgs),

    /// Entrypoint for the prepare-commit-msg hook.
    #[command(hide = true)]
    Run(RunArgs),
}

#[derive(Args, Debug)]
struct InstallArgs {
    /// Install hook even if a prepare-commit-msg exists already.
    #[arg(long)]
    overwrite: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum CommitSource {
    Message,
    Template,
    Merge,
    Squash,
    Commit,
}

impl fmt::Display for CommitSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message => write!(f, "message"),
            Self::Template => write!(f, "template"),
            Self::Merge => write!(f, "merge"),
            Self::Squash => write!(f, "squash"),
            Self::Commit => write!(f, "commit"),
        }
    }
}

#[derive(Args, Debug)]
struct RunArgs {
    /// Path to the file that holds the commit message so far.
    path: PathBuf,

    /// Type of commit.
    #[clap(value_enum)]
    commit: Option<CommitSource>,

    /// Commit SHA-1 if this is an amended commit.
    hash: Option<String>,
}

fn install(args: &InstallArgs) -> Result<(), Error> {
    let repository = git::Repository::open()?;
    if repository.repo.is_bare() {
        log::warn!("Repository is bare; not installing hook");
        return Err(Error::Exit(ExitCode::from(1)));
    }
    let hook_path = repository.hook_path();
    if hook_path.exists() {
        if !args.overwrite {
            log::error!(
                "Hook already exists at {}",
                util::truncate_path(&hook_path).display()
            );
            log::info!("Use `bureaucrat install --overwrite` to install hook anyways");
            return Err(Error::Exit(ExitCode::from(1)));
        }
        log::debug!("Overwriting existing hook");
    }
    repository.install_hook().map_err(Error::Io)?;
    log::info!(
        "Hook installed at {}",
        util::truncate_path(&hook_path).display()
    );
    Ok(())
}

fn run(args: &RunArgs) -> Result<(), Error> {
    match &args.commit {
        None => log::debug!("Tagging unspecified commit type"),
        Some(CommitSource::Template) => log::debug!("Tagging 'template' type commit"),
        Some(commit) => {
            log::debug!("Skipping tagging for '{}' type commit", commit);
            return Ok(());
        }
    }

    let repository = git::Repository::open()?;
    let config_file_path = repository.discover_config()?;
    let config = config::Config::load(config_file_path)?;
    log::debug!(
        "Using codes {:?} for branches {:?}",
        config.codes,
        config.branch_prefixes
    );

    let branch = repository.current_branch()?;
    let Some(reference) = parse::find_issue_reference(&config, &branch) else {
        log::debug!("No issue reference found in '{}'", &branch);
        return Ok(());
    };

    let contents = fs::read_to_string(&args.path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => Error::FileNotFound(args.path.clone()),
        _ => Error::Io(error),
    })?;
    let mut temp_file = NamedTempFile::new().map_err(Error::Io)?;
    write!(temp_file, "\n\n{}{}", reference, contents).map_err(Error::Io)?;
    if let Err(error) = temp_file.persist(&args.path) {
        log::warn!("Could not ovewrite commit message: {}", error);
    };
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder()
        .filter_level(cli.log_level)
        .init();

    let result = match &cli.command {
        Commands::Install(args) => install(args),
        Commands::Run(args) => run(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error::Exit(exit_code)) => exit_code,
        Err(Error::FileNotFound(path)) => {
            log::error!("File not found: {}", path.display());
            ExitCode::from(1)
        }
        Err(Error::InvalidConfiguration(error)) => {
            log::warn!("Configuration could not be parsed: {}", error);
            ExitCode::SUCCESS
        }
        Err(Error::Io(error)) => {
            log::error!("IO error: {}", error);
            ExitCode::from(1)
        }
        Err(Error::NoBranch) => {
            log::warn!("Branch doesn't exist yet");
            ExitCode::SUCCESS
        }
        Err(Error::NoConfigurationFile) => {
            log::warn!("No configuration file was found");
            ExitCode::SUCCESS
        }
        Err(Error::NoRepository) => {
            log::error!("Could not find repository");
            ExitCode::from(1)
        }
        Err(Error::UnknownGit(error)) => {
            log::error!("Error while accessing repository: {}", error);
            ExitCode::from(1)
        }
    }
}
