use crate::config;
use anyhow::Result;
use clap::Args;
use divvee::task::Task;
use divvee::*;
use log::debug;
use std::path::Path;
use tempfile::Builder;

#[derive(Args, Debug)]
pub struct CreateCmd {
    title: String,
    #[arg(long, short = 't')]
    team: Option<String>,
    #[arg(long, short = 'd')]
    description: Option<String>,
}

pub async fn run(dv: &mut System, args: CreateCmd) -> Result<()> {
    let team = match args.team {
        Some(team) => team,
        None => config::default_team()?,
    };
    let rel_dir = Path::new(&team).join("tasks");
    debug!("create rel_dir: {}", rel_dir.display());

    let mut new_task: Task = dv
        .read_doc(rel_dir.join("_task.md"))
        .ok()
        .unwrap_or_else(Task::default);
    new_task.title = args.title;
    if let Some(description) = args.description {
        if !description.is_empty() {
            new_task.description = Some(description);
        }
    } else {
        let template = new_task
            .to_doc_string()
            .replace("\n---\n", "\n# Provide description below dashed line\n---");
        let editted = edit::edit_with_builder(template, Builder::new().suffix(".md"))?;
        new_task = Task::parse_doc(&editted, None)?;
    }

    let id = dv.next_id(&rel_dir)?;
    let team_id = format!("{}-{}", &team, id);
    let path = rel_dir.join(&team_id).with_extension("md");
    debug!("Creating doc at {}", path.display());
    let doc = dv.create_doc(path, new_task).await?;

    let task = doc.read_doc::<Task>()?;
    println!("Created {}", task.id().unwrap());
    Ok(())
}
