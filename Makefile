.PHONY: build
build:
	cargo build

.PHONY: build-release
build-release:
	cargo lint && cargo build --release

.PHONY: examples
examples:
	cargo build --examples

.PHONY: test-and-publish
test-and-publish:
	cargo test && cargo publish

.PHONY: run-embeded
run-embeded:
	cargo build --examples && ./scripts/xephyr.sh

.PHONY: check-all
check-all:
	cargo test
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets
	cargo rustdoc --all-features -- -D warnings


# GitHub helpers using the official gh GitHub CLI
.PHONY: pr
pr:
	gh pr create

.PHONY: list-prs
list-prs:
	gh pr list

.PHONY: list-issues
list-issues:
	gh issue list

.PHONY: new-issue
new-issue:
	gh issue create
