.PHONY: build
build:
	cargo build

.PHONY: build-release
build-release:
	cargo lint && cargo build --release

.PHONY: check-all
check-all:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --all-features --examples --tests
	cargo rustdoc --all-features -- -D warnings
	cargo test

.PHONY: doc
doc:
	cargo doc --open &

.PHONY: examples
examples:
	cargo build --examples

.PHONY: run-embeded
run-embeded:
	cargo build --examples && ./scripts/xephyr.sh

.PHONY: test-and-publish
test-and-publish:
	cargo test && cargo publish


# GitHub helpers using the official gh GitHub CLI
.PHONY: list-issues
list-issues:
	gh issue list

.PHONY: list-prs
list-prs:
	gh pr list

.PHONY: new-issue
new-issue:
	gh issue create

.PHONY: pr
pr:
	gh pr create
