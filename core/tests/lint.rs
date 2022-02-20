use chrono::{Duration, Local};
use tasktree_core::Tree;
use toml::from_str;

#[test]
fn lint_succeeds() {
	let mut tree: Tree = from_str(include_str!("good.toml")).unwrap();
	tree.populate_tree().unwrap();
	tree.lint_tree().unwrap();
}

#[test]
fn lint_fails_in_past() {
	let mut tree: Tree = from_str(include_str!("past.toml")).unwrap();
	tree.populate_tree().unwrap();
	assert!(tree.lint_tree().is_err());
}

#[test]
fn lint_fails_not_enough_time() {
	let mut tree: Tree = from_str(include_str!("good.toml")).unwrap();
	tree.tasks.get_mut("a").unwrap().due = Some(Local::now() + Duration::seconds(1));
	tree.populate_tree().unwrap();
	assert!(tree.lint_tree().is_err());
}
