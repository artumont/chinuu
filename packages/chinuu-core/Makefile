# shellcheck disable=SC2046,SC2035

PKG   ?= chinuu-core
CARGO ?= cargo

.DEFAULT_GOAL := help

##@ Meta

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"} \
		/^[a-zA-Z0-9_-]+:.*##/ {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2} \
		/^##@/ {printf "\n\033[1m%s\033[0m\n", substr($$0, 5)}' $(MAKEFILE_LIST)

##@ Check

.PHONY: check test lint format format-check doc
check: format-check lint test ## Run every gate
test: ## Run the crate test suite
	$(CARGO) test -p $(PKG) --all-targets
lint: ## Run clippy, warnings are errors
	$(CARGO) clippy -p $(PKG) --all-targets -- -D warnings
format: ## Format the crate sources in place
	$(CARGO) fmt -p $(PKG)
format-check: ## Fail if the crate sources are not formatted
	$(CARGO) fmt -p $(PKG) -- --check
doc: ## Build and open the API docs
	$(CARGO) doc -p $(PKG) --no-deps --open

##@ Clean

.PHONY: clean
clean: ## Remove this crate's build output
	$(CARGO) clean -p $(PKG)
