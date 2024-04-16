use anyhow::Result;
use clap::Args;
use divvee::{task::Task, System};
use log::debug;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct ReindexCmd {
    /// ID or Team (ID-prefix) to reindex
    target: String,
}
pub async fn run(dv: &mut System, args: ReindexCmd) -> Result<()> {
    let mut split = args.target.split('-');

    let team = split.next().unwrap();
    let base_dir = PathBuf::from(team).join("tasks");

    if let Some(id_num) = split.next() {
        let path = base_dir.join(format!("{team}-{id_num}.md"));
        debug!("Reindexing {}", path.display());
        dv.reindex::<Task>(&path).await?;
    } else {
        debug!("Processing {}", base_dir.display());
        for doc in dv.read_dir(&base_dir)? {
            if let Some(fname) = doc.repo_path().file_name() {
                if fname.to_string_lossy().starts_with(&team) {
                    debug!("Reindexing {}", doc.repo_path().display());
                    dv.reindex::<Task>(&doc.repo_path()).await?;
                }
            }
        }
    }

    println!("Indexing complete");
    Ok(())
}
