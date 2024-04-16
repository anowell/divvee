use crate::{config, print::print_tasks, FilterArgs, StatusFilter, View};
use anyhow::{bail, Result};
use clap::Args;
use divvee::*;

#[derive(Args, Debug)]
pub struct ListCmd {
    #[command(flatten)]
    filters: FilterArgs,
    #[arg(long, short = 'v', default_value_t=View::Line)]
    view: View,
    // #[command(flatten)]
    // labels: Labels,
}

impl ListCmd {
    pub fn mine() -> ListCmd {
        let assignee = config::me().email.clone();
        let filters = FilterArgs {
            assignee: Some(assignee),
            status: Some(StatusFilter::Open),
            ..FilterArgs::default()
        };
        ListCmd {
            filters,
            view: View::Line,
        }
    }
}

pub async fn run(dv: &mut System, args: ListCmd) -> Result<()> {
    // let team = match args.filters.team {
    //     Some(team) => Some(team),
    //     None => config::default_team().ok(),
    // };

    let tasks = dv.query(&args.filters.to_where_clause()).await?;
    if tasks.is_empty() {
        bail!("No tasks matching query");
    }

    if tasks.is_empty() {
        println!("No tasks found matching filter.")
    } else {
        print_tasks(&tasks, args.view);
    }

    Ok(())
}
