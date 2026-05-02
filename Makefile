.PHONY: setup
setup: .git/hooks

.git/hooks: scripts/git_hooks
	ln -s ../scripts/git_hooks .git/hooks
