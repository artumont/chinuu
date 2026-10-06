# shellcheck disable=SC2046,SC2035

PKG  ?= @chinuu/editor
PNPM ?= pnpm

RUN := $(PNPM) --filter $(PKG) run

.DEFAULT_GOAL := help

##@ Meta

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"} \
		/^[a-zA-Z0-9_-]+:.*##/ {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2} \
		/^##@/ {printf "\n\033[1m%s\033[0m\n", substr($$0, 5)}' $(MAKEFILE_LIST)

##@ Check

.PHONY: check lint typecheck format format-check
check: format-check lint typecheck ## Run every gate
lint: ## Lint with eslint, including the type-aware rules
	$(RUN) lint
typecheck: ## Type-check with tsc, no emit
	$(RUN) typecheck
format: ## Format sources with prettier, in place
	$(RUN) format
format-check: ## Fail if sources are not formatted
	$(PNPM) --filter $(PKG) exec prettier --check .

##@ Test

.PHONY: test-browser
test-browser: ## Serve the test fixture and open it in a browser (not headless)
	$(RUN) test:browser

##@ Clean

.PHONY: clean
clean: ## Remove installed dependencies and the vite cache
	rm -rf -- node_modules
