#[cfg(target_os = "linux")]
use landlock::{
	ABI, Access, AccessFs,
	Ruleset, RulesetAttr, RulesetCreatedAttr,
	RulesetStatus, RulesetError,
	path_beneath_rules,
};

use log;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
pub fn landlock_restrict(rw_dirs: &[&PathBuf], r_dirs: &[&PathBuf]) {
	let abi = ABI::V1;
	let read_dirs = [
		"/usr", "/etc", "/dev",
	];
	let all_dirs = [
		"/dev/tty", "/dev/null",
		"/tmp",
	];
	let status = Ruleset::default()
		.handle_access(AccessFs::from_all(abi)).unwrap()
		.create().unwrap()
		.add_rules(path_beneath_rules(&read_dirs, AccessFs::from_read(abi))).unwrap()
		.add_rules(path_beneath_rules(r_dirs, AccessFs::from_read(abi))).unwrap()
		.add_rules(path_beneath_rules(&all_dirs, AccessFs::from_all(abi))).unwrap()
		.add_rules(path_beneath_rules(rw_dirs, AccessFs::from_all(abi))).unwrap()
		.restrict_self().unwrap();
	match status.ruleset {
		RulesetStatus::FullyEnforced => log::info!("Landlock fully enforced"),
		RulesetStatus::PartiallyEnforced => log::warn!("Landlock partially enforced"),
		RulesetStatus::NotEnforced => log::warn!("Landlock unenforced"),
	}
}

#[cfg(not(target_os = "linux"))]
pub fn landlock_restrict(_rw_dirs: &[&PathBuf], _r_dirs: &[&PathBuf]) {}
