use std::{ops::Add, path::PathBuf, str::FromStr, time::Duration};

use chrono::NaiveDateTime;
use tasktree_core::Config;

lazy_static::lazy_static! {
	pub static ref DEFAULT_TASKSETS_PATH: PathBuf = if cfg!(windows) {
		let mut path = PathBuf::from_str(
			&std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default".to_string()),
		)
		.unwrap();
		path.push("\\Documents\\tasktree");
		path
	} else if cfg!(target_os = "macos") {
		let mut path =
			PathBuf::from_str(&std::env::var("HOME").unwrap_or_else(|_| "/Users/Default".to_string()))
				.unwrap();
		path.push("Documents/tasktree");
		path
	} else {
		let mut path =
			PathBuf::from_str(&std::env::var("XDG_DOCUMENTS_DIR").unwrap_or_else(|_| std::env::var("HOME").unwrap_or_else(|_| ".".to_string()) + "/Documents"))
				.unwrap();
		path.push("tasktree");
		path
	};
}

fn parse_and_unwrap_duration(s: &str) -> Duration {
	parse_duration::parse(s).unwrap_or_else(|_| panic!("Could not parse duration: {}", s))
}

fn parse_and_unwrap_fuzzydate(s: &str) -> NaiveDateTime {
	fuzzydate::parse(s).unwrap_or_else(|_| panic!("Could not parse date: {}", s))
}

#[derive(StructOpt, Debug)]
pub struct GlobalArgs {
	#[structopt(long = "tasksets-path", short = "T", env = "TASKTREE_TASKSETS_PATH", default_value=DEFAULT_TASKSETS_PATH.to_str().unwrap(), help = "Path to the directory containing the task sets.\n")]
	pub tasksets_path: PathBuf,
	#[structopt(
		short,
		long,
		env = "TASKTREE_DEFAULT_TASKSET",
		default_value = "default",
		help = "The task to run. It should correspond to a file file located at $TASKSETS/$TASKSET.toml.\n"
	)]
	pub taskset: Vec<String>,
	#[structopt(
		long = "pomodoro-length",
		short = "p",
		env = "TASKTREE_POMODORO_LENGTH",
		default_value = "20 minutes",
		help = "The length of a pomodoro session.\n",
		parse(from_str = parse_and_unwrap_duration)
	)]
	pub pomodoro_length: Duration,
	#[structopt(
		long = "short-break-length",
		short = "b",
		env = "TASKTREE_SHORT_BREAK_LENGTH",
		default_value = "5 minutes",
		help = "The length of a short break.\n",
		parse(from_str = parse_and_unwrap_duration)
	)]
	pub short_break_length: Duration,
	#[structopt(
		long = "long-break-length",
		short = "B",
		env = "TASKTREE_LONG_BREAK_LENGTH",
		default_value = "15 minutes",
		help = "The length of a long break.\n",
		parse(from_str = parse_and_unwrap_duration)
	)]
	pub long_break_length: Duration,
	#[structopt(
		long = "long-break-frequency",
		short = "f",
		env = "TASKTREE_LONG_BREAK_FREQUENCY",
		default_value = "4",
		help = "The number of pomodoros before a long break.\n"
	)]
	pub long_break_frequency: u32,
	#[structopt(subcommand)]
	pub cmd: TaskTree,
}

impl Add<&GlobalArgs> for Config {
	type Output = Config;

	fn add(self, rhs: &GlobalArgs) -> Self::Output {
		let mut config = self;
		// This should update the config with the values from the command line.
		config.pomodoro_length = rhs.pomodoro_length;
		config.short_break_length = rhs.short_break_length;
		config.long_break_length = rhs.long_break_length;
		config.long_break_after = rhs.long_break_frequency;
		config
	}
}

#[derive(StructOpt, Debug)]
pub enum TaskTree {
	#[structopt(name = "license", about = "Prints the license information.")]
	License,
	#[structopt(
		name = "list-tasks",
		about = "Lists all tasks in the selected task set."
	)]
	ListTasks,
	#[structopt(name = "show-tree", about = "Prints the tree of tasks.")]
	ShowTree,
	#[structopt(name = "add-task", about = "Add a task.")]
	AddTask {
		#[structopt(help = "The name of the task.\n")]
		name: String,
		#[structopt(help = "The description of the task.\n")]
		description: String,
		#[structopt(
			long = "duration",
			short = "t",
			help = "The expected duration of the task.\n",
			parse(from_str = parse_and_unwrap_duration)
		)]
		duration: Option<Duration>,
		#[structopt(long = "depends", short = "r", help = "A dependency of the task.\n")]
		depends_on: Vec<String>,
		#[structopt(
			long = "symbolic",
			short = "s",
			help = "The task is symbolic; it is complete when its dependencies are complete.\n"
		)]
		symbolic: bool,
		#[structopt(
			long = "complete",
			short = "c",
			help = "The task is already complete.\n"
		)]
		complete: bool,
		#[structopt(long = "due", short = "d", help = "The due date of the task.\n", parse(from_str = parse_and_unwrap_fuzzydate))]
		due: Option<NaiveDateTime>,
	},
	#[structopt(name = "remove-task", about = "Remove a task.")]
	RemoveTask {
		#[structopt(help = "The name of the task.\n")]
		name: Vec<String>,
	},
	#[structopt(name = "complete-task", about = "Complete a task.")]
	CompleteTask {
		#[structopt(
			short = "c",
			long = "complete",
			help = "Whether the task is complete\n"
		)]
		complete: Option<bool>,
		#[structopt(help = "The name of the task.\n")]
		name: Vec<String>,
	},
	#[structopt(name = "show-task", about = "Show a task.")]
	ShowTask {
		#[structopt(help = "The name of the task.\n")]
		name: Vec<String>,
	},
	#[structopt(name = "lint", about = "Lint the task tree.")]
	Lint,
}
