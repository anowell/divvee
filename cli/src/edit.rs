use crate::{config, util, Status};
use anyhow::Result;
use clap::Args;
use divvee::task::Task;
use divvee::*;
use log::debug;
use std::path::Path;
use tempfile::Builder;

#[derive(Args, Debug)]
pub struct EditCmd {
    id: String,
    #[arg(long, short = 'a')]
    assignee: Option<String>,
    #[arg(long, short = 's', value_enum)]
    status: Option<Status>,
    #[arg(long, short = 'i')]
    interactive: bool,
    // #[arg(long, short = 't')]
    // team: Option<String>,
    #[arg(long, short = 'm')]
    title: Option<String>,
    #[arg(long, short = 'd')]
    description: Option<String>,
    // labels: Option<Vec<String>>,
}

pub async fn run(dv: &mut System, mut args: EditCmd) -> Result<()> {
    let (team, id) = util::team_and_id(&args.id)?;

    let rel_dir = Path::new(&team).join("tasks");
    debug!("edit rel_dir: {}", rel_dir.display());

    let fname = Path::new(&id).with_extension("md");
    let doc = dv.load(rel_dir.join(fname))?;
    let mut task = doc.read_doc::<Task>()?;
    let original = task.clone();

    if let Some(title) = args.title.take() {
        task.title = title;
    }

    if let Some(assignee) = args.assignee.take() {
        task.assignee = match &*assignee {
            "" => None,
            "me" => Some(config::me().email.to_owned()),
            _ => Some(assignee),
        }
    }

    if let Some(description) = args.description.take() {
        task.description = match description.is_empty() {
            true => None,
            false => Some(description),
        }
    }

    if let Some(status) = args.status.take() {
        task.status = Some(status.to_string());
    }

    // if let Some(team) = args.team.take() {
    //     TODO: move file
    //     lookup new dir
    //     get next available id in dir
    //     perform a repo file move
    //     create a symlink in the original file's location
    //     (also provide a tool that does batch cleanup of references/symlinks)
    // }

    if args.interactive {
        let template = task.to_doc_string();
        let editted = edit::edit_with_builder(template, Builder::new().suffix(".md"))?;
        task = Task::parse_doc(&editted, None)?;
    }

    if task != original {
        let doc = dv.update_doc(doc.repo_path(), task).await?;
        let task = doc.read_doc::<Task>()?;
        println!("Updated {}", task.id().unwrap());
    } else {
        println!("No changes were made");
    }

    Ok(())
}
