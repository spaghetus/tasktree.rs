use std::fs;

use chrono::Local;
use chrono::TimeZone;
use ptree::graph::print_graph;
use tasktree_core::{Task, Tree};

use crate::loader;

pub fn add_task(args: crate::args::GlobalArgs) {
	if let crate::args::TaskTree::AddTask {
		name,
		description,
		duration,
		depends_on,
		complete,
		due,
		symbolic,
	} = args.cmd
	{
		if let [taskset_name] = &args.taskset[..] {
			let path = args
				.tasksets_path
				.clone()
				.join(taskset_name.to_owned() + ".toml");
			let mut taskset: Tree = toml::from_str(
				&fs::read_to_string(path.clone()).unwrap_or_else(|_| "tasks = {}".to_string()),
			)
			.expect("refusing to overwrite invalid taskset, please check manually");
			let mut task = Task::default();
			task.description = description;
			task.estimated_time = duration;
			task.depends_on = depends_on;
			task.complete = complete;
			task.due = due.map(|due| Local.from_local_datetime(&due).unwrap());
			task.symbolic = symbolic;
			taskset.tasks.insert(name.to_owned(), task);
			taskset.populate_tree().unwrap();
			fs::write(path, toml::to_string(&taskset).unwrap()).expect("Couldn't write taskset");
		} else {
			panic!("exactly one taskset must be specified for add_task.")
		}
	} else {
		unreachable!()
	}
}

pub fn remove_task(args: crate::args::GlobalArgs) {
	if let crate::args::TaskTree::RemoveTask { name } = args.cmd {
		for taskset_name in &args.taskset {
			let path = args
				.tasksets_path
				.clone()
				.join(taskset_name.to_owned() + ".toml");
			let mut taskset: Tree = toml::from_str(
				&fs::read_to_string(path.clone()).unwrap_or_else(|_| "tasks = {}".to_string()),
			)
			.expect("refusing to overwrite invalid taskset, please check manually");
			for task in &name {
				taskset.tasks.remove(task);
			}
			taskset.populate_tree().unwrap();
			fs::write(path, toml::to_string(&taskset).unwrap()).expect("Couldn't write taskset");
		}
	} else {
		unreachable!()
	}
}

pub fn complete_task(args: crate::args::GlobalArgs) {
	if let crate::args::TaskTree::CompleteTask { name, complete } = args.cmd {
		for taskset_name in &args.taskset {
			let path = args
				.tasksets_path
				.clone()
				.join(taskset_name.to_owned() + ".toml");
			let mut taskset: Tree = toml::from_str(
				&fs::read_to_string(path.clone()).unwrap_or_else(|_| "tasks = {}".to_string()),
			)
			.expect("refusing to overwrite invalid taskset, please check manually");
			for task in &name {
				if let Some(task) = taskset.tasks.get_mut(task) {
					task.complete = complete.unwrap_or(true);
				}
			}
			taskset.populate_tree().unwrap();
			fs::write(path, toml::to_string(&taskset).unwrap()).expect("Couldn't write taskset");
		}
	} else {
		unreachable!()
	}
}

pub fn show_task(args: crate::args::GlobalArgs) {
	if let crate::args::TaskTree::ShowTask { name } = args.cmd {
		for taskset_name in &args.taskset {
			let path = args
				.tasksets_path
				.clone()
				.join(taskset_name.to_owned() + ".toml");
			let taskset: Tree = toml::from_str(
				&fs::read_to_string(path.clone()).unwrap_or_else(|_| "tasks = {}".to_string()),
			)
			.expect("refusing to overwrite invalid taskset, please check manually");
			for task in &name {
				if let Some(task) = taskset.tasks.get(task) {
					println!(
						"{}",
						serde_json::to_string_pretty(&task).unwrap_or_else(|_| "".to_string())
					);
				}
			}
		}
	} else {
		unreachable!()
	}
}

pub fn show_tree(args: crate::args::GlobalArgs) {
	if let crate::args::TaskTree::ShowTree = args.cmd {
		let task_tree = loader::load_tasksets(&args);
		let indices = task_tree.tree.node_indices().collect::<Vec<_>>();
		let root = indices
			.iter()
			.find(|&&i| task_tree.tree.node_weight(i).unwrap().is_root)
			.unwrap();
		print_graph(&task_tree.tree, *root).unwrap();
	} else {
		unreachable!()
	}
}

pub fn lint(args: crate::args::GlobalArgs) {
	let task_tree = loader::load_tasksets(&args);
	let result = task_tree.lint_tree();
	if let Err(errors) = result {
		for error in errors {
			println!("{:#?}", error);
		}
	} else {
		println!("no errors found.");
	}
}
