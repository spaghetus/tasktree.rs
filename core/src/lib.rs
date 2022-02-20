//! A library for tasktree

use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Local};
use solvent::DepGraph;
use thiserror::Error;

#[macro_use]
extern crate serde;
#[macro_use]
extern crate no_panic;

/// Configuration for tasktree.
/// Should be stored in a file at `$XDG_CONFIG_HOME/tasktree.toml`.
#[repr(C)]
#[non_exhaustive]
#[derive(Serialize, Deserialize)]
pub struct Config {
	/// The length of the pomodoro timer.
	pub pomodoro_length: Duration,
	/// The length of a short break.
	pub short_break_length: Duration,
	/// The length of a long break.
	pub long_break_length: Duration,
	/// The number of pomodoro sessions before a long break.
	pub long_break_after: u32,
}

/// A task tree.
#[repr(C)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Default)]
pub struct Tree {
	/// The tasks in the tree.
	pub tasks: HashMap<String, Task>,
	/// The generated dependency tree.
	#[serde(skip)]
	pub tree: DepGraph<String>,
}

impl Tree {
	/// Populate the generated dependency tree.
	pub fn populate_tree(&mut self) -> Result<(), anyhow::Error> {
		// Populate dependency tree
		for (name, task) in &self.tasks {
			self.tree
				.register_dependencies(name.clone(), task.depends_on.clone());
			if task.complete {
				self.tree.mark_as_satisfied(&[name.clone()])?;
			}
		}
		Ok(())
	}
	/// Lint the tree.
	pub fn lint_tree(&self) -> Result<(), anyhow::Error> {
		// Detect floating symbolic tasks
		{
			let mut good_symbolic_tasks = vec![];
			// Find every good symbolic
			loop {
				let mut found_any_new = false;
				for (name, task) in &self.tasks {
					if task.depends_on.is_empty() {
						continue;
					}
					if (task.depends_on.iter().any(|dep| !self.tasks[dep].symbolic)
						|| task
							.depends_on
							.iter()
							.any(|dep| good_symbolic_tasks.contains(dep)))
						&& !good_symbolic_tasks.contains(&name)
					{
						good_symbolic_tasks.push(name.clone());
						found_any_new = true;
					}
				}
				if !found_any_new {
					break;
				}
			}
			// Find any bad symbolic
			for (name, task) in &self.tasks {
				if task.symbolic && !good_symbolic_tasks.contains(name) {
					return Err(TaskTreeCoreError::FloatingSymbolic {
						task_name: name.clone(),
					}
					.into());
				}
			}
		}
		// Detect impossible tasks
		{
			let now = Local::now();
			for (name, task) in &self.tasks {
				if task.due.is_none() {
					continue;
				}
				let due_date = task.due.unwrap();
				if due_date < now && !task.complete {
					return Err(TaskTreeCoreError::ImpossibleTaskError {
						task_name: name.clone(),
						reason: ImpossibleTaskReason::DueInPast,
					}
					.into());
				}
				let mut completion_time = task.estimated_time.unwrap_or_default();
				completion_time += self
					.tree
					.dependencies_of(&name)?
					.flatten()
					.map(|dep| self.tasks[dep].estimated_time.unwrap_or_default())
					.sum();
				// cast because chrono's duration is different from std's duration smh
				let completion_time = chrono::Duration::seconds(completion_time.as_secs() as i64);
				if (now + completion_time) > due_date {
					return Err(TaskTreeCoreError::ImpossibleTaskError {
						task_name: name.clone(),
						reason: ImpossibleTaskReason::NotEnoughTime,
					}
					.into());
				}
			}
		}
		Ok(())
	}
}

#[repr(C)]
#[non_exhaustive]
#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum TaskTreeCoreError {
	#[error("task is impossible given constraints")]
	ImpossibleTaskError {
		task_name: String,
		reason: ImpossibleTaskReason,
	},
	#[error("a symbolic task must have at least one non-symbolic dependency")]
	FloatingSymbolic { task_name: String },
}

/// The reason why a task is impossible.
#[repr(C)]
#[non_exhaustive]
#[derive(Debug)]
pub enum ImpossibleTaskReason {
	/// The sum of the estimated completion times of the task and its dependencies extends past the due date.
	NotEnoughTime,
	/// The task is due in the past, so it cannot be completed.
	DueInPast,
}

#[repr(C)]
#[non_exhaustive]
#[derive(Serialize, Deserialize)]
pub struct Task {
	pub description: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub estimated_time: Option<Duration>,
	pub depends_on: Vec<String>,
	pub symbolic: bool,
	pub complete: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub due: Option<DateTime<Local>>,
}
