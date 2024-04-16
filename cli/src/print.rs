use crate::{Status, View};
use divvee::task::Task;
use owo_colors::{OwoColorize, Style};
use std::cmp::max;
use termimad::MadSkin;

pub fn print_tasks(tasks: &[Task], view: View) {
    match view {
        View::Id => {
            for id in tasks.iter().filter_map(|t| t.id()) {
                println!("{id}");
            }
        }
        View::Line => {
            let widths = tasks.iter().fold([0; 3], |acc, t| {
                let w = widths(t);
                [max(acc[0], w[0]), max(acc[1], w[1]), max(acc[2], w[2])]
            });
            print_header_line(widths);
            for task in tasks {
                print_task_line(task, widths);
            }
        }
        View::Detail => {
            for task in tasks {
                print_task_detail(task, false);
                println!("---");
            }
        }
        View::Json => println!("{}", serde_json::to_string(&tasks).unwrap()),
    }
}

fn widths(task: &Task) -> [usize; 3] {
    [
        task.id().map(|s| s.len()).unwrap_or(0),
        task.assignee.as_ref().map(|s| s.len()).unwrap_or(0),
        task.title.len(),
    ]
}

pub fn print_task(task: &Task, view: View) {
    match view {
        View::Id => println!("{}", task.id().unwrap()),
        View::Line => print_task_line(task, widths(task)),
        View::Detail => print_task_detail(task, true),
        View::Json => println!("{}", serde_json::to_string(&task).unwrap()),
    }
}
fn print_header_line(widths: [usize; 3]) {
    let [id_w, assignee_w, title_w] = widths;
    let s = format!(
        "  {:id_w$}  {:assignee_w$}  {:title_w$}",
        "ID", "Assignee", "Title",
    );
    println!("{}", s.underline());
}
fn print_task_line(task: &Task, widths: [usize; 3]) {
    let [id_w, assignee_w, title_w] = widths;
    let status = task.status.as_ref().and_then(|s| s.parse::<Status>().ok());
    let status_sym = status.map(|s| s.to_sym()).unwrap_or(' ');

    let style = match status.unwrap_or(Status::Todo) {
        Status::Todo => Style::new(),
        Status::InProgress => Style::new().bold(),
        Status::Done => Style::new().green(),
        Status::Canceled => Style::new().dimmed().strikethrough(),
        Status::Duplicate => Style::new().dimmed(),
    };
    let s = format!(
        "{status_sym} {:id_w$}  {:assignee_w$}  {:title_w$}",
        task.id().unwrap(),
        task.assignee.as_deref().unwrap_or(""),
        task.title,
    );
    println!("{}", s.style(style));
}

trait OrNa {
    fn or_na(&self) -> &str;
}

impl OrNa for Option<String> {
    fn or_na(&self) -> &str {
        self.as_deref().unwrap_or("n/a")
    }
}

fn print_task_detail(task: &Task, print_description: bool) {
    println!(
        "{}, {}",
        task.id().unwrap().bold(),
        task.title.bold().green()
    );
    println!("Status: {}", task.status.or_na().bold());
    println!("Assignee: {}", task.assignee.or_na());

    if print_description {
        if let Some(description) = &task.description {
            let skin = MadSkin::default();
            println!("\n{}", skin.term_text(description));
        }
    }
}
