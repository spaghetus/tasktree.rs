use std::fs;

use tasktree_core::Tree;

use crate::args::GlobalArgs;

pub fn load_tasksets(args: &GlobalArgs) -> Tree {
	let mut tree = Tree::default();
	let base_path = args.tasksets_path.clone();
	for set_name in args.taskset.iter() {
		let set_path = base_path.join(set_name.clone() + ".toml");
		let set = toml::from_str::<Tree>(&fs::read_to_string(set_path).unwrap_or_else(|e| {
			eprintln!("Skipping taskset file due to {}", e);
			"".to_string()
		}))
		.unwrap_or_else(|e| {
			eprintln!("Skipping taskset file due to {}", e);
			Tree::default()
		});
		tree += &set;
	}
	tree
}
