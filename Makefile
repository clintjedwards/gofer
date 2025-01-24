# Make settings
# Mostly copied from: https://tech.davis-hansson.com/p/make/

# Use Bash
SHELL := bash

# If one of the commands fails just fail properly and don't run the other commands.
.SHELLFLAGS := -eu -o pipefail -c

# Allows me to use a single shell session so you can do things like 'cd' without doing hacks.
.ONESHELL:

# Tells make not to do crazy shit.
MAKEFLAGS += --no-builtin-rules

# Allows me to replace tabs with > characters. This makes it a bit easier to use things like forloops in bash.
ifeq ($(origin .RECIPEPREFIX), undefined)
  $(error This Make does not support .RECIPEPREFIX. Please use GNU Make 4.0 or later)
endif
.RECIPEPREFIX = >

# Colors
COLOR_GREEN=\033[0;32m
COLOR_RED=\033[0;31m
COLOR_BLUE=\033[0;34m
COLOR_END=\033[0m

SEMVER = 0.0.0

## generate-openapi-backend: build json documents for openapi
generate-openapi-backend:
> cd gofer
> cargo run --bin generate_openapi

## generate-openapi-sdk: build json documents for openapi
generate-openapi-sdk:
> cd sdk
> oapi-codegen -generate "types,client" -response-type-suffix Resp -package sdk ../gofer/docs/src/assets/openapi.json > go/sdk.go
> oapi-codegen -generate "types" -response-type-suffix Resp -package extensions openapi.json > go/extensions/sdk.go
> cargo run --bin generate_openapi_sdk

## generate-openapi
generate-openapi: generate-openapi-backend generate-openapi-sdk

## run: build and run Gofer web service
run:
> @$(MAKE) -j run-tailwind run-backend
.PHONY: run

run-backend:
> cd gofer
# This will only set the debug log level is there isn't one set by the environment. Helps with local debugging.
> export GOFER_WEB_API__LOG_LEVEL=$${GOFER_WEB_API__LOG_LEVEL:-debug}
> cargo run --bin gofer -- service start

## build-release: build Gofer for release.
build-release: generate-openapi build-docs
> cd gofer
> cargo build --release --target=x86_64-unknown-linux-gnu
> cd ..
> mv ./target/x86_64-unknown-linux-gnu/release/gofer ./target/x86_64-unknown-linux-gnu/release/gofer_amd64_linux_gnu

## run-docs: build and run documentation website for development
run-docs:
> @bash -c 'trap "touch book/html/.gitkeep" EXIT; cd gofer/docs && mdbook serve --open'
.PHONY: run-docs

## run-integration-tests: Run integration tests using hurl.dev
run-integration-tests: run-hurl-tests cleanup-integration-tests

## run-hurl-tests: Run integration tests using hurl.dev
run-hurl-tests:
> @rm -rf /tmp/gofer* || true
> @pkill -9 gofer || true
> @cd gofer
> @export GOFER_WEB_API__LOG_LEVEL=debug
> @export GOFER_WEB_DEVELOPMENT__BYPASS_AUTH=false
> @cargo run --bin gofer -- service start > /dev/null 2>&1 &

> echo -n "Waiting for server to start responding..."
> @while ! curl -o /dev/null -s -H "gofer-api-version: v0" --fail --connect-timeout 5 http://localhost:8080/api/system/metadata; do
> 	@sleep 1;
> done;

> @cd tests
> SECRET=$$(curl -s --fail --request POST \
  --url http://localhost:8080/api/tokens/bootstrap \
  --header 'gofer-api-version: v0' \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' | jq -r '.secret')
> echo -ne "\r\033[K"  # Moves cursor to start of line and clears the line
> echo "Hurl Results"
> echo "--------------------------------"
> hurl --color --test *.hurl --variable secret=$$SECRET --error-format long

## cleanup-integration-tests: Clean up the background gofer process.
cleanup-integration-tests:
> @pkill -9 gofer

## run-tailwind: Run the tailwind compiler
run-tailwind:
> npx tailwindcss@3.4.17 -i ./gofer/src/main.css -o ./gofer/public/css/main.css --watch >/dev/null 2>&1

## clippy-pedantic: Let clippy nitpick your code.
clippy-pedantic:
> cargo clippy --all -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings

## build-docs: build final documentation site artifacts
build-docs:
> cd gofer/docs
> mkdir -p book/html
> mdbook build
> touch book/html/.gitkeep
.PHONY: build-docs

## build-containers: build containers
build-containers: check-semver-included
> cd containers
> echo -e "$(COLOR_BLUE)Building Cron Extension$(COLOR_END)"
> docker build -f extensions/cron/Dockerfile -t ghcr.io/clintjedwards/gofer/extensions/cron:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/extensions/cron:${SEMVER} ghcr.io/clintjedwards/gofer/extensions/cron:latest

> echo -e "$(COLOR_BLUE)Building Interval Extension$(COLOR_END)"
> docker build -f extensions/interval/Dockerfile -t ghcr.io/clintjedwards/gofer/extensions/interval:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/extensions/interval:${SEMVER} ghcr.io/clintjedwards/gofer/extensions/interval:latest

> echo -e "$(COLOR_BLUE)Building Github Extension$(COLOR_END)"
> docker build -f extensions/github/Dockerfile -t ghcr.io/clintjedwards/gofer/extensions/github:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/extensions/github:${SEMVER} ghcr.io/clintjedwards/gofer/extensions/github:latest

> echo -e "$(COLOR_BLUE)Building Debug Container Envs$(COLOR_END)"
> docker build -f debug/envs/Dockerfile -t ghcr.io/clintjedwards/gofer/debug/envs:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/debug/envs:${SEMVER} ghcr.io/clintjedwards/gofer/debug/envs:latest

> echo -e "$(COLOR_BLUE)Building Debug Container Fail$(COLOR_END)"
> docker build -f debug/fail/Dockerfile -t ghcr.io/clintjedwards/gofer/debug/fail:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/debug/fail:${SEMVER} ghcr.io/clintjedwards/gofer/debug/fail:latest

> echo -e "$(COLOR_BLUE)Building Debug Container Log$(COLOR_END)"
> docker build -f debug/log/Dockerfile -t ghcr.io/clintjedwards/gofer/debug/log:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/debug/log:${SEMVER} ghcr.io/clintjedwards/gofer/debug/log:latest

> echo -e "$(COLOR_BLUE)Building Debug Container Wait$(COLOR_END)"
> docker build -f debug/wait/Dockerfile -t ghcr.io/clintjedwards/gofer/debug/wait:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/debug/wait:${SEMVER} ghcr.io/clintjedwards/gofer/debug/wait:latest

> echo -e "$(COLOR_BLUE)Building Tooling Container Golang$(COLOR_END)"
> docker build -f tools/Dockerfile_golang -t ghcr.io/clintjedwards/gofer/tools:golang .

> echo -e "$(COLOR_BLUE)Building Tooling Container Rust$(COLOR_END)"
> docker build -f tools/Dockerfile_rust -t ghcr.io/clintjedwards/gofer/tools:rust .

## push-containers: push containers to github
push-containers: check-semver-included
> echo -e "$(COLOR_BLUE)Push Cron Extension Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/extensions/cron:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/extensions/cron:latest
> echo -e "$(COLOR_BLUE)Push Internal Extension Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/extensions/interval:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/extensions/interval:latest

> echo -e "$(COLOR_BLUE)Push Github Extension Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/extensions/github:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/extensions/github:latest

> echo -e "$(COLOR_BLUE)Push Debug Env Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/debug/envs:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/debug/envs:latest

> echo -e "$(COLOR_BLUE)Push Debug Fail Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/debug/fail:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/debug/fail:latest

> echo -e "$(COLOR_BLUE)Push Debug Log Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/debug/log:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/debug/log:latest

> echo -e "$(COLOR_BLUE)Push Debug Wait Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/debug/wait:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/debug/wait:latest

> echo -e "$(COLOR_BLUE)Push Tooling Golang Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/tools:golang

> echo -e "$(COLOR_BLUE)Push Tooling Rust Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/tools:rust


## help: prints this help message
help:
> @echo "Usage: "
> @sed -n 's/^##//p' ${MAKEFILE_LIST} | column -t -s ':' |  sed -e 's/^/ /'
.PHONY: help

check-semver-included:
ifeq ($(SEMVER), 0.0.0)
>	$(error SEMVER is undefined; ex. SEMVER=0.0.1)
endif
