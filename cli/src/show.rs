use crate::print::print_task;
use crate::{util, View};
use anyhow::{bail, Result};
use clap::Args;
use divvee::task::Task;
use divvee::*;
use log::debug;
use std::io;
use std::path::Path;

#[derive(Args, Debug)]
pub struct ShowCmd {
    id: String,

    #[arg(long, short = 'v', default_value_t=View::Detail)]
    view: View,
}

pub fn run(dv: &mut System, args: ShowCmd) -> Result<()> {
    let (team, id) = util::team_and_id(&args.id)?;
    let rel_dir = Path::new(&team).join("tasks");
    debug!("show rel_dir: {}", rel_dir.display());

    let path = rel_dir.join(&id).with_extension("md");
    match dv.read_doc_with_meta::<Task, _>(path) {
        Ok(task) => print_task(&task, args.view),
        Err(divvee::Error::IoError(err)) if err.kind() == io::ErrorKind::NotFound => {
            bail!("{} not found", id);
        }
        Err(err) => {
            return Err(err.into());
        }
    }
    Ok(())
}
