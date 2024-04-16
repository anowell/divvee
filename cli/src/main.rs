use anyhow::{bail, Result};
use clap::{arg, Arg, ArgMatches, Args, Command, FromArgMatches, Parser, Subcommand, ValueEnum};
use config::Config;
use create::CreateCmd;
use derive_more::Display;
use divvee::System;
use edit::EditCmd;
use env_logger::Env;
use list::ListCmd;
use log::debug;
use reindex::ReindexCmd;
use show::ShowCmd;
use std::path::PathBuf;
use std::{env, str};

mod config;
mod create;
mod edit;
mod list;
mod print;
mod reindex;
mod show;
mod util;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    // /// Disables reading defaults from environment or config file
    // #[arg(long)]
    // no_defaults: bool
    /// Path to the document repo. Defaults to DIVVEE_REPO or current dir
    #[arg(short, long)]
    repo: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    Create(CreateCmd),
    List(ListCmd),
    Show(ShowCmd),
    Edit(EditCmd),
    // Search(SearchCmd),
    // Sync,
    // Link(LinkCmd),
    BulkEdit(BulkEditCmd),
    Reindex(ReindexCmd),
}

#[derive(Args, Debug, Default)]
struct FilterArgs {
    #[arg(long, short = 't')]
    team: Option<String>,
    #[arg(long, short = 'a')]
    assignee: Option<String>,
    #[arg(long, short = 's', value_enum)]
    status: Option<StatusFilter>,
    #[command(flatten)]
    labels: Labels,
}

impl FilterArgs {
    fn to_where_clause(&self) -> String {
        let mut parts = Vec::new();
        if let Some(team) = &self.team {
            parts.push(format!("id LIKE '{team}-%'"));
        }
        if let Some(assignee) = &self.assignee {
            parts.push(format!("assignee = '{assignee}'"));
        }
        if let Some(status) = self.status {
            let clause = match status {
                StatusFilter::Open => {
                    "(status IS null OR status = 'Todo' OR status = 'In Progress')"
                }
                StatusFilter::InProgress => "status = 'In Progress'",
                StatusFilter::Closed => {
                    "(status = 'Done' OR status = 'Canceled' OR status ='Duplicate')"
                }
            };
            parts.push(clause.to_owned());
        }
        match parts.is_empty() {
            true => String::from("1 = 1"),
            false => parts.join(" AND "),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Labels(Vec<String>);

impl FromArgMatches for Labels {
    fn from_arg_matches(matches: &ArgMatches) -> std::result::Result<Self, clap::Error> {
        let mut labels = Labels(Vec::new());
        labels.update_from_arg_matches(matches)?;
        Ok(labels)
    }
    fn update_from_arg_matches(
        &mut self,
        matches: &ArgMatches,
    ) -> std::result::Result<(), clap::Error> {
        let labels: Vec<String> = ('A'..='Z')
            .filter_map(|c| {
                matches
                    .get_one::<String>(&format!("{}-label", c))
                    .map(|label| format!("{}-{}", c, label))
            })
            .collect();

        // if labels.is_empty() {
        //     return Err(clap::error::Error::raw(
        //         clap::error::ErrorKind::MissingRequiredArgument,
        //         "Expected label argument",
        //     ));
        // }

        self.0 = labels;
        Ok(())
    }
}

impl Args for Labels {
    fn augment_args(cmd: Command) -> Command {
        let cmd = cmd.clone();
        Self::augment_args_for_update(cmd)
    }
    fn augment_args_for_update(cmd: Command) -> Command {
        let args: Vec<Arg> = ('A'..='Z')
            .map(|c| Arg::new(&format!("{}-label", c)).short(c).hide(true))
            .collect();
        cmd.args(args)
    }
}

// TODO: consider using tantivity or the grep crate powering ripgrep
#[derive(Args, Debug)]
struct SearchCmd {
    query: String,
    // regex: bool,
}

// #[derive(Args, Debug)]
// struct AddCmd {
//     labels: Option<Vec<String>>,
//     props: Option<BTreeMap<String>>,
// }

// #[derive(Args, Debug)]
// struct RmCmd {
//     labels: Option<Vec<String>>,
//     props: Option<Vec<String>>,
// }

#[derive(Args, Debug)]
struct BulkEditCmd {
    query: String,
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    edit_opts: EditCmd,
    // add_opts: AddCmd,
    // rm_opts: RemoveCmd,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, Display)]
enum Status {
    #[display(fmt = "Todo")]
    Todo,
    #[display(fmt = "In Progress")]
    InProgress,
    #[display(fmt = "Done")]
    Done,
    #[display(fmt = "Canceled")]
    Canceled,
    #[display(fmt = "Duplicate")]
    Duplicate,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum StatusFilter {
    Open,
    InProgress,
    Closed,
}

impl str::FromStr for Status {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Status> {
        match s {
            "Todo" => Ok(Status::Todo),
            "In Progress" => Ok(Status::InProgress),
            "Done" => Ok(Status::Done),
            "Canceled" => Ok(Status::Canceled),
            "Duplicate" => Ok(Status::Duplicate),
            _ => bail!("Unknown status {}", s),
        }
    }
}

impl Status {
    fn to_sym(&self) -> char {
        match self {
            Status::Todo => '○',
            Status::InProgress => '◐',
            Status::Done => '✓',
            Status::Canceled => '𐄂',
            Status::Duplicate => '=',
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, Display)]
enum View {
    #[display(fmt = "id")]
    Id,
    #[display(fmt = "line")]
    Line,
    #[display(fmt = "detail")]
    Detail,
    #[display(fmt = "json")]
    Json,
}

#[tokio::main]
async fn main() -> Result<()> {
    Config::init()?;
    let cli = Cli::parse();
    let default_filter = match cli.debug {
        0 => "warn",
        1 => "warn,dv=debug",
        2 => "warn,dv=debug,divvee=debug",
        3 => "debug",
        _ => "trace",
    };
    let env = Env::new().filter_or("DIVVEE_LOG", default_filter);
    env_logger::Builder::from_env(env).init();
    debug!("{:#?}", cli);
    debug!("{:#?}", Config::get());

    let repo_path = match cli.repo {
        Some(p) => p,
        None => match env::var("DIVVEE_REPO") {
            Ok(p) => PathBuf::from(p),
            Err(_) => env::current_dir()?,
        }
    };
    let mut dv = System::init(repo_path).await?;
    match cli.cmd {
        None => list::run(&mut dv, ListCmd::mine()).await?,
        Some(Cmd::Create(args)) => create::run(&mut dv, args).await?,
        Some(Cmd::Edit(args)) => edit::run(&mut dv, args).await?,
        Some(Cmd::Show(args)) => show::run(&mut dv, args)?,
        Some(Cmd::List(args)) => list::run(&mut dv, args).await?,
        Some(Cmd::Reindex(args)) => reindex::run(&mut dv, args).await?,
        _ => unimplemented!("Command not implemented"),
    }
    Ok(())
}
