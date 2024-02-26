// The Licensed Work is (c) 2022 Sygma
// SPDX-License-Identifier: LGPL-3.0-only

use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

fn main() {
	generate_cargo_keys();

	rerun_if_git_head_changed();
}
