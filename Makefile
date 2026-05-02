.PHONY: setup
setup: .git/hooks

.git/hooks: scripts/git_hooks
	ln -s ../scripts/git_hooks .git/hooks


.PHONY: fmt
fmt:
	cargo clippy --all-targets --fix --allow-dirty
	cargo fmt
