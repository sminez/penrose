.PHONY: audit-dependencies
audit-dependencies:
	cargo audit

.PHONY: build
build:
	cargo build

.PHONY: build-release
build-release:
	cargo lint && cargo build --release

.PHONY: clippy
clippy:
	cargo clippy --workspace --all-targets --all-features --examples --tests

.PHONY: check-all
check-all:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --all-features --examples --tests
	cargo rustdoc --all-features -- -D warnings
	cargo test --all-features

.PHONY: doc
doc:
	cargo doc --all-features --open &

.PHONY: examples
examples:
	cargo build --examples

.PHONY: run-embeded
run-embeded:
	@cargo build --examples && ./scripts/xephyr.sh

.PHONY: test
test:
	cargo test --lib

.PHONY: test-and-publish
test-and-publish:
	cargo test --all-features && cargo publish

.PHONY: upgrade-check
upgrade-check:
	cargo upgrade --workspace --dry-run

.PHONY: todo
todo:
	rg 'TODO|FIXME|todo!' crates examples src tests


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
