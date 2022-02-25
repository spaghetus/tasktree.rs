use structopt::StructOpt;
use tasktree_core::Config;
pub mod args;
pub mod loader;
pub mod task_edit;

#[macro_use]
extern crate structopt;

fn main() {
	let all_opt = args::GlobalArgs::from_args();
	let _config = Config::default() + &all_opt;
	match all_opt.cmd {
		args::TaskTree::License => println!(include_str!("../../LICENSE")),
		args::TaskTree::ListTasks => {
			let tasks = loader::load_tasksets(&all_opt);
			println!("{}", serde_json::to_string_pretty(&tasks.tasks).unwrap());
		}
		args::TaskTree::AddTask { .. } => task_edit::add_task(all_opt),
		args::TaskTree::RemoveTask { .. } => task_edit::remove_task(all_opt),
		args::TaskTree::ShowTree => task_edit::show_tree(all_opt),
		args::TaskTree::CompleteTask { .. } => task_edit::complete_task(all_opt),
		args::TaskTree::ShowTask { .. } => task_edit::show_task(all_opt),
		args::TaskTree::Lint => task_edit::lint(all_opt),
	}
}
