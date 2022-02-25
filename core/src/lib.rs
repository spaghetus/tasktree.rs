//! A library for tasktree

use std::{
	collections::{HashMap, HashSet},
	fmt::Display,
	ops::{Add, AddAssign},
	time::Duration,
};

use chrono::{DateTime, Local};
use petgraph::{graph::NodeIndex, Graph};
use thiserror::Error;

#[macro_use]
extern crate serde;

/// Configuration for tasktree.
/// Should be stored in a file at `$XDG_CONFIG_HOME/tasktree.toml`.
#[repr(C)]
#[non_exhaustive]
#[derive(Serialize, Deserialize)]
pub struct Config {
	/// The number of pomodoro sessions before a long break.
	pub long_break_after: u32,
	/// The length of the pomodoro timer.
	pub pomodoro_length: Duration,
	/// The length of a short break.
	pub short_break_length: Duration,
	/// The length of a long break.
	pub long_break_length: Duration,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			pomodoro_length: Duration::from_secs(20 * 60),
			short_break_length: Duration::from_secs(5 * 60),
			long_break_length: Duration::from_secs(15 * 60),
			long_break_after: 4,
		}
	}
}

#[derive(Clone)]
pub struct TaskNode {
	pub name: String,
	pub complete: bool,
	pub is_root: bool,
}

impl Display for TaskNode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if self.is_root {
			write!(f, "root")
		} else {
			write!(f, "{}: {}", self.name, self.complete)
		}
	}
}

/// A task tree.
#[repr(C)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Tree {
	/// The tasks in the tree.
	pub tasks: HashMap<String, Task>,
	/// The generated dependency tree.
	#[serde(skip)]
	pub tree: Graph<TaskNode, bool>,
}

impl Tree {
	/// Populate the generated dependency tree.
	pub fn populate_tree(&mut self) -> Result<(), anyhow::Error> {
		self.tree = Graph::new();
		// Resolve symbolics
		let mut did_something = false;
		while did_something {
			did_something = false;
			let mut completed = vec![];
			for (name, task) in self.tasks.iter() {
				if task.symbolic && !task.complete {
					let tasks = self.tasks.clone();
					let implied = task.depends_on.iter().all(|name| tasks[name].complete);
					if implied {
						completed.push(name.to_owned());
					}
					did_something = did_something || implied;
				}
			}
			for name in completed {
				self.tasks.get_mut(&name).unwrap().complete = true;
			}
		}
		// Populate dependency tree
		// Create root node
		let root = self.tree.add_node(TaskNode {
			name: "root".to_string(),
			complete: true,
			is_root: true,
		});
		// Create all nodes
		let indices: HashMap<String, NodeIndex> = self
			.tasks
			.iter()
			.map(|(name, task)| {
				let index = self.tree.add_node(TaskNode {
					name: name.clone(),
					complete: task.complete,
					is_root: false,
				});
				self.tree.add_edge(root, index, false);
				(name.clone(), index)
			})
			.collect();
		// Add edges
		for (name, task) in self.tasks.iter() {
			for dep in task.depends_on.iter() {
				let from = indices[name];
				let to = indices[dep];
				self.tree.add_edge(from, to, self.tasks[dep].complete);
			}
		}
		Ok(())
	}
	/// Lint the tree.
	pub fn lint_tree(&self) -> Result<(), Vec<anyhow::Error>> {
		let mut errors = vec![];
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
						&& !good_symbolic_tasks.contains(name)
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
					errors.push(
						TaskTreeCoreError::FloatingSymbolic {
							task_name: name.clone(),
						}
						.into(),
					);
				}
			}
		}
		// Detect cyclic
		let cyclic = {
			fn visitor(
				tasks: &HashMap<String, Task>,
				task: String,
				visited: &Vec<String>,
			) -> HashSet<(String, String)> {
				let mut visited = visited.clone();
				visited.push(task.clone());
				for i in &tasks[&task].depends_on {
					if visited.contains(&i) {
						return {
							let mut set = HashSet::new();
							set.insert((task.clone(), i.clone()));
							set
						};
					}
				}
				tasks[&task]
					.depends_on
					.iter()
					.map(|n| visitor(tasks, n.clone(), &visited))
					.flatten()
					.collect()
			}

			let found: HashSet<_> = self
				.tasks
				.iter()
				.map(|n| visitor(&self.tasks, n.0.clone(), &vec![]))
				.flatten()
				.collect();
			for err in &found {
				errors.push(
					TaskTreeCoreError::CyclicDependency {
						task_name: err.0.clone(),
						dependency: err.1.clone(),
					}
					.into(),
				);
			}
			!found.is_empty()
		};
		// Detect impossible tasks due to duration
		if !cyclic {
			let now = Local::now();
			for (name, task) in &self.tasks {
				if task.due.is_none() {
					continue;
				}
				let due_date = task.due.unwrap();
				if due_date < now && !task.complete {
					errors.push(
						TaskTreeCoreError::ImpossibleTaskError {
							task_name: name.clone(),
							reason: ImpossibleTaskReason::DueInPast,
						}
						.into(),
					);
				}
				let mut completion_time = task.estimated_time.unwrap_or_default();
				fn visitor(
					tasks: &HashMap<String, Task>,
					task: String,
					completion_time: &mut std::time::Duration,
				) {
					if !tasks[&task].complete {
						*completion_time =
							*completion_time + tasks[&task].estimated_time.unwrap_or_default();
						for dep in tasks[&task].depends_on.iter() {
							visitor(tasks, dep.clone(), completion_time);
						}
					}
				}
				visitor(&self.tasks, name.clone(), &mut completion_time);
				// coerce because chrono's duration is different from std's duration smh
				let completion_time = chrono::Duration::from_std(completion_time).unwrap();
				if (now + completion_time) > due_date {
					errors.push(
						TaskTreeCoreError::ImpossibleTaskError {
							task_name: name.clone(),
							reason: ImpossibleTaskReason::NotEnoughTime,
						}
						.into(),
					);
				}
			}
		}
		// Detect missing dependencies
		{
			for (name, task) in &self.tasks {
				for dependency in &task.depends_on {
					if !self.tasks.contains_key(dependency) {
						errors.push(
							TaskTreeCoreError::NonexistentDependency {
								task_name: name.clone(),
								dependency: dependency.clone(),
							}
							.into(),
						);
					}
				}
			}
		}
		if !errors.is_empty() {
			Err(errors)
		} else {
			Ok(())
		}
	}
}

impl Add<&Tree> for Tree {
	type Output = Self;
	fn add(mut self, other: &Tree) -> Self {
		self.tasks
			.extend(other.tasks.iter().map(|(k, v)| (k.clone(), v.clone())));
		self.populate_tree()
			.expect("failed to repopulate tree after concat");
		self
	}
}

impl AddAssign<&Tree> for Tree {
	fn add_assign(&mut self, other: &Tree) {
		*self = self.clone() + other;
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
	#[error("nonexistent dependency")]
	NonexistentDependency {
		task_name: String,
		dependency: String,
	},
	#[error("cyclic dependency")]
	CyclicDependency {
		task_name: String,
		dependency: String,
	},
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
#[derive(Serialize, Deserialize, Clone, Default)]
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
