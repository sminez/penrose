.PHONY: build
build:
	cargo build

.PHONY: build-release
build-release:
	cargo build --release

.PHONY: watch
watch:
	git ls-files | entr -ac cargo test
