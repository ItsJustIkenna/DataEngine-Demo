# DataEngine Build System
# Provides comprehensive build management for all environments

# Main Makefile for DataEngine project
.PHONY: help build release optimized debug clean test lint bench install dev watchelm-dev: ## Deploy DataEngine with Helm (development)
	@echo -e "$(CYAN)🚀 Deploying DataEngine with Helm (development)...$(NC)"
	@echo -e "$(YELLOW)💡 Set environment variables for custom secrets:$(NC)"
	@echo -e "$(GRAY)   export DB_PASSWORD='your-dev-password'$(NC)"
	@echo -e "$(GRAY)   export REDIS_PASSWORD='your-redis-password'$(NC)"
	@echo -e "$(GRAY)   export KRAKEN_API_KEY='your-api-key'$(NC)"
	@./scripts/deploy.ps1 -Environment dev
	@echo -e "$(GREEN)✅ Development environment deployed$(NC)"l
.DEFAULT_GOAL := help

# Project configuration
PROJECT_NAME := DataEngine
CARGO := cargo
PYTHON := python3
SHELL := /bin/bash

# Colors for output
CYAN    := \033[0;36m
GREEN   := \033[0;32m
YELLOW  := \033[1;33m
RED     := \033[0;31m
GRAY    := \033[0;37m
WHITE   := \033[1;37m
NC      := \033[0m

# Detect OS
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
    OS := linux
endif
ifeq ($(UNAME_S),Darwin)
    OS := macos
endif
ifeq ($(OS),Windows_NT)
    OS := windows
endif

help: ## Show this help message
	@echo -e "$(CYAN)🚀 $(PROJECT_NAME) Build System$(NC)"
	@echo -e "$(CYAN)=============================$(NC)"
	@echo ""
	@echo -e "$(GREEN)Available targets:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "$(WHITE)  %-12s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo -e "$(GREEN)Profile-specific builds:$(NC)"
	@echo -e "$(WHITE)  debug       $(NC) Debug build (default)"
	@echo -e "$(WHITE)  release     $(NC) Release build"
	@echo -e "$(WHITE)  optimized   $(NC) Optimized profile build"
	@echo ""
	@echo -e "$(GREEN)Examples:$(NC)"
	@echo -e "$(GRAY)  make release test    # Release build with tests$(NC)"
	@echo -e "$(GRAY)  make optimized      # Optimized profile build$(NC)"
	@echo -e "$(GRAY)  make clean all      # Clean build with all checks$(NC)"

debug: ## Build in debug mode
	@echo -e "$(CYAN)🔧 Building in debug mode...$(NC)"
	@$(CARGO) build
	@echo -e "$(GREEN)✅ Debug build completed$(NC)"

release: ## Build in release mode
	@echo -e "$(CYAN)🔧 Building in release mode...$(NC)"
	@$(CARGO) build --release
	@echo -e "$(GREEN)✅ Release build completed$(NC)"

optimized: ## Build with the optimized profile
	@echo -e "$(CYAN)🔧 Building with additional optimizations...$(NC)"
	@echo -e "$(YELLOW)⚡ Configuring profile flags...$(NC)"
	@export RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2,+sse4.2 -C lto=fat -C codegen-units=1" && \
	export CARGO_PROFILE_RELEASE_LTO="fat" && \
	export CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1" && \
	export CARGO_PROFILE_RELEASE_PANIC="abort" && \
	export CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS="false" && \
	$(CARGO) build --release
	@echo -e "$(GREEN)✅ Optimized build completed$(NC)"

build: debug ## Alias for debug build

clean: ## Clean build artifacts
	@echo -e "$(YELLOW)🧹 Cleaning build artifacts...$(NC)"
	@$(CARGO) clean
	@echo -e "$(GREEN)✅ Clean completed$(NC)"

test: ## Run all tests
	@echo -e "$(CYAN)🧪 Running tests...$(NC)"
	@$(CARGO) test --lib --verbose
	@$(CARGO) test --test "*" --verbose
	@$(CARGO) test --doc --verbose
	@echo -e "$(GREEN)✅ All tests completed$(NC)"

test-unit: ## Run unit tests only
	@echo -e "$(CYAN)🧪 Running unit tests...$(NC)"
	@$(CARGO) test --lib
	@echo -e "$(GREEN)✅ Unit tests completed$(NC)"

test-integration: ## Run integration tests only
	@echo -e "$(CYAN)🔗 Running integration tests...$(NC)"
	@$(CARGO) test --test "*"
	@echo -e "$(GREEN)✅ Integration tests completed$(NC)"

lint: ## Run linting and formatting checks
	@echo -e "$(CYAN)🔍 Running code quality checks...$(NC)"
	@echo -e "$(YELLOW)📎 Running Clippy...$(NC)"
	@$(CARGO) clippy --all-targets --all-features -- -D warnings
	@echo -e "$(YELLOW)🎨 Checking formatting...$(NC)"
	@$(CARGO) fmt --check
	@echo -e "$(GREEN)✅ Linting completed$(NC)"

format: ## Format code
	@echo -e "$(CYAN)🎨 Formatting code...$(NC)"
	@$(CARGO) fmt
	@echo -e "$(GREEN)✅ Code formatted$(NC)"

bench: ## Run benchmarks
	@echo -e "$(CYAN)📊 Running benchmarks...$(NC)"
	@$(CARGO) bench
	@echo -e "$(GREEN)✅ Benchmarks completed$(NC)"

doc: ## Generate documentation
	@echo -e "$(CYAN)📚 Generating documentation...$(NC)"
	@$(CARGO) doc --no-deps --open
	@echo -e "$(GREEN)✅ Documentation generated$(NC)"

install: ## Install dependencies
	@echo -e "$(CYAN)📦 Installing dependencies...$(NC)"
	@rustup update
	@$(CARGO) fetch
	@echo -e "$(GREEN)✅ Dependencies installed$(NC)"

dev: ## Development build with watch mode
	@echo -e "$(CYAN)👀 Starting development mode with file watching...$(NC)"
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -x 'build' -x 'test --lib'; \
	else \
		echo -e "$(RED)❌ cargo-watch not installed. Installing...$(NC)"; \
		$(CARGO) install cargo-watch; \
		cargo watch -x 'build' -x 'test --lib'; \
	fi

watch: dev ## Alias for dev target

check: ## Quick check without building
	@echo -e "$(CYAN)⚡ Running quick check...$(NC)"
	@$(CARGO) check
	@echo -e "$(GREEN)✅ Check completed$(NC)"

all: clean optimized test lint ## Clean build with all checks
	@echo -e "$(GREEN)🚀 Complete build pipeline finished!$(NC)"

# Advanced targets
profile: ## Profile the application
	@echo -e "$(CYAN)📊 Profiling application...$(NC)"
	@$(CARGO) build --release
	@echo -e "$(YELLOW)Run profiling tools manually on target/release/program$(NC)"

security: ## Run security audit
	@echo -e "$(CYAN)🔒 Running security audit...$(NC)"
	@if command -v cargo-audit >/dev/null 2>&1; then \
		cargo audit; \
	else \
		echo -e "$(YELLOW)⚠️ cargo-audit not installed. Installing...$(NC)"; \
		$(CARGO) install cargo-audit; \
		cargo audit; \
	fi
	@echo -e "$(GREEN)✅ Security audit completed$(NC)"

deps: ## Show dependency tree
	@echo -e "$(CYAN)🌳 Dependency tree:$(NC)"
	@$(CARGO) tree

update: ## Update dependencies
	@echo -e "$(CYAN)🔄 Updating dependencies...$(NC)"
	@$(CARGO) update
	@echo -e "$(GREEN)✅ Dependencies updated$(NC)"

size: ## Show binary size analysis
	@echo -e "$(CYAN)📏 Binary size analysis:$(NC)"
	@if [ -f "target/release/program" ]; then \
		ls -lh target/release/program; \
		if command -v bloaty >/dev/null 2>&1; then \
			bloaty target/release/program; \
		else \
			echo -e "$(YELLOW)💡 Install bloaty for detailed size analysis$(NC)"; \
		fi; \
	else \
		echo -e "$(RED)❌ No release binary found. Run 'make release' first$(NC)"; \
	fi

# Platform-specific targets
linux: ## Linux-specific optimizations
ifeq ($(OS),linux)
	@echo -e "$(CYAN)🐧 Linux-specific build...$(NC)"
	@export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld" && $(CARGO) build --release
else
	@echo -e "$(YELLOW)⚠️ Not running on Linux$(NC)"
endif

macos: ## macOS-specific optimizations
ifeq ($(OS),macos)
	@echo -e "$(CYAN)🍎 macOS-specific build...$(NC)"
	@export RUSTFLAGS="-C target-cpu=native" && $(CARGO) build --release
else
	@echo -e "$(YELLOW)⚠️ Not running on macOS$(NC)"
endif

windows: ## Windows-specific optimizations
ifeq ($(OS),windows)
	@echo -e "$(CYAN)🪟 Windows-specific build...$(NC)"
	@set RUSTFLAGS=-C target-cpu=native && $(CARGO) build --release
else
	@echo -e "$(YELLOW)⚠️ Not running on Windows$(NC)"
endif

# Kubernetes/Helm targets
helm-dev: ## Deploy DataEngine with Helm (development)
	@echo -e "$(CYAN)� Deploying DataEngine with Helm (development)...$(NC)"
	@./scripts/deploy.ps1 -Environment dev
	@echo -e "$(GREEN)✅ Development environment deployed$(NC)"

helm-prod: ## Deploy DataEngine with Helm (production)
	@echo -e "$(CYAN)� Deploying DataEngine with Helm (prod)...$(NC)"
	@./scripts/deploy.ps1 -Environment prod
	@echo -e "$(GREEN)✅ Prod Helm environment deployed$(NC)"

helm-stop: ## Stop/uninstall Helm deployment
	@echo -e "$(YELLOW)� Stopping Helm deployment...$(NC)"
	@./scripts/deploy.ps1 -Stop

helm-status: ## Show Helm deployment status
	@echo -e "$(CYAN)📊 Checking Helm deployment status...$(NC)"
	@./scripts/deploy.ps1 -Status

helm-logs: ## Show logs from Helm deployment
	@echo -e "$(CYAN)� Showing logs...$(NC)"
	@./scripts/logs.ps1 -Follow

helm-clean: ## Clean Helm deployment and redeploy
	@echo -e "$(YELLOW)🧹 Clean Helm redeployment...$(NC)"
	@./scripts/deploy.ps1 -Clean

# Convenience aliases
dev: helm-dev ## Alias for helm-dev
prod: helm-prod ## Alias for helm-prod  
deploy: helm-dev ## Alias for helm-dev
stop: helm-stop ## Alias for helm-stop
status: helm-status ## Alias for helm-status
logs: helm-logs ## Alias for helm-logs

# Information targets
version: ## Show version information
	@echo -e "$(CYAN)📋 Version Information:$(NC)"
	@echo -e "$(WHITE)Project: $(PROJECT_NAME)$(NC)"
	@echo -e "$(WHITE)Rust version: $(shell rustc --version)$(NC)"
	@echo -e "$(WHITE)Cargo version: $(shell cargo --version)$(NC)"
	@echo -e "$(WHITE)OS: $(OS)$(NC)"

env: ## Show build environment
	@echo -e "$(CYAN)🌍 Build Environment:$(NC)"
	@echo -e "$(WHITE)CARGO: $(CARGO)$(NC)"
	@echo -e "$(WHITE)RUSTFLAGS: $${RUSTFLAGS:-<not set>}$(NC)"
	@echo -e "$(WHITE)CARGO_PROFILE_RELEASE_LTO: $${CARGO_PROFILE_RELEASE_LTO:-<not set>}$(NC)"
	@echo -e "$(WHITE)Working directory: $(PWD)$(NC)"

# Docker targets
docker-build: ## Build Docker image
	@echo -e "$(CYAN)🐳 Building Docker image...$(NC)"
	@cd .. && docker build -f DataEngine/Dockerfile -t dataengine:latest .
	@echo -e "$(GREEN)✅ Docker image built successfully$(NC)"

docker-run: ## Run Docker container
	@echo -e "$(CYAN)🐳 Running Docker container...$(NC)"
	docker run -p 8080:8080 -p 9090:9090 dataengine:latest

docker-compose-up: ## Start full stack with docker-compose
	@echo -e "$(CYAN)🐳 Starting docker-compose stack...$(NC)"
	docker-compose up -d
	@echo -e "$(GREEN)✅ Docker stack started$(NC)"

docker-compose-down: ## Stop docker-compose stack
	@echo -e "$(CYAN)🐳 Stopping docker-compose stack...$(NC)"
	docker-compose down
	@echo -e "$(GREEN)✅ Docker stack stopped$(NC)"

docker-logs: ## Show docker-compose logs
	docker-compose logs -f

docker: docker-build docker-compose-up ## Build and run with docker-compose
