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

# Allows me to replace tabs with > characters. This makes the things a bit easier to use things like forloops in bash.
ifeq ($(origin .RECIPEPREFIX), undefined)
  $(error This Make does not support .RECIPEPREFIX. Please use GNU Make 4.0 or later)
endif
.RECIPEPREFIX = >

# Colors

COLOR_GREEN=\033[0;32m
COLOR_RED=\033[0;31m
COLOR_BLUE=\033[0;34m
COLOR_END=\033[0m

# App Vars

APP_NAME = gofer
GIT_COMMIT = $(shell git rev-parse --short HEAD)
# Although go 1.18 has the git info baked into the binary now it still seems like there is no support
# For including outside variables except this. So keep it for now.
GO_LDFLAGS = '-X "github.com/clintjedwards/${APP_NAME}/internal/cli.appVersion=$(VERSION)" \
				-X "github.com/clintjedwards/${APP_NAME}/internal/api.appVersion=$(VERSION)"'
SHELL = /bin/bash
SEMVER = 0.0.0
VERSION = ${SEMVER}_${GIT_COMMIT}

## build: run tests and compile application
build: check-path-included check-semver-included build-protos build-sdk
> go test ./...
> go mod tidy
> export CGO_ENABLED=1
> go build -ldflags $(GO_LDFLAGS) -o $(OUTPUT)
.PHONY: build

## build-protos: build protobufs
build-protos:
> protoc --proto_path=proto --go_out=proto/go --go_opt=paths=source_relative \
	 --go-grpc_out=proto/go --go-grpc_opt=paths=source_relative proto/*.proto
> cd proto/rust
> cargo build --release
.PHONY: build-protos

## build-sdk: build rust sdk
build-sdk:
> cd sdk/rust
> cargo build
.PHONY: build-sdk

## run: build application and run server
run:
> export GOFER_LOG_LEVEL=debug
> export GOFER_DEBUG=true
> export GOFER_DEV_MODE=true
> export SEMVER=0.0.0
> go build -ldflags $(GO_LDFLAGS) -o /tmp/${APP_NAME}
> /tmp/${APP_NAME} service start
.PHONY: run

## run-race: build application and run server with race detector
run-race:
> export GOFER_DEBUG=true
> export GOFER_DEV_MODE=true
> export SEMVER=0.0.0
> go build -race -ldflags $(GO_LDFLAGS) -o /tmp/${APP_NAME}
> /tmp/${APP_NAME} service start
.PHONY: run-race

## run-docs: build and run documentation website for development
run-docs:
> cd documentation
> mdbook serve --open
.PHONY: run-docs

## build-docs: build final documentation site artifacts
build-docs:
> cd documentation
> mdbook build
.PHONY: build-docs

## push-docs: push docs to github
push-docs:
> git checkout main
> rm .gitignore
> cd documentation
> mdbook build
> git add ./book/html/*
> git commit -m "docs update"
> cd ..
> git subtree split --prefix documentation/book/html -b gh-pages
> git push -f origin gh-pages:gh-pages
> git branch -D gh-pages
> git reset --hard origin/main
.PHONY: push-docs

# 	docker build -f extensions/github/Dockerfile -t ghcr.io/clintjedwards/gofer/extensions/github:${semver} .
#	docker tag ghcr.io/clintjedwards/gofer/extensions/github:${semver} ghcr.io/clintjedwards/gofer/extensions/github:latest
#	docker push ghcr.io/clintjedwards/gofer/extensions/github:${semver}
#	docker push ghcr.io/clintjedwards/gofer/extensions/github:latest

## build-containers: build docker containers
build-containers: check-semver-included
> cd containers
> echo -e "$(COLOR_BLUE)Building Cron Extension$(COLOR_END)"
> docker build -f extensions/cron/Dockerfile -t ghcr.io/clintjedwards/gofer/extensions/cron:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/extensions/cron:${SEMVER} ghcr.io/clintjedwards/gofer/extensions/cron:latest

> echo -e "$(COLOR_BLUE)Building Interval Extension$(COLOR_END)"
> docker build -f extensions/interval/Dockerfile -t ghcr.io/clintjedwards/gofer/extensions/interval:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/extensions/interval:${SEMVER} ghcr.io/clintjedwards/gofer/extensions/interval:latest

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

> echo -e "$(COLOR_BLUE)Building Common Task Container Debug$(COLOR_END)"
> docker build -f tasks/debug/Dockerfile -t ghcr.io/clintjedwards/gofer/tasks/debug:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/tasks/debug:${SEMVER} ghcr.io/clintjedwards/gofer/tasks/debug:latest

## push-containers: push docker containers to github
push-containers: check-semver-included
> echo -e "$(COLOR_BLUE)Push Cron Extension Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/extensions/cron:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/extensions/cron:latest
> echo -e "$(COLOR_BLUE)Push Internal Extension Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/extensions/interval:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/extensions/interval:latest

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

> echo -e "$(COLOR_BLUE)Push Debug Common Task Container$(COLOR_END)"
> docker push ghcr.io/clintjedwards/gofer/tasks/debug:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/tasks/debug:latest

## help: prints this help message
help:
> @echo "Usage: "
> @sed -n 's/^##//p' ${MAKEFILE_LIST} | column -t -s ':' |  sed -e 's/^/ /'
.PHONY: help

check-path-included:
ifndef OUTPUT
>	$(error OUTPUT is undefined; ex. OUTPUT=/tmp/${APP_NAME})
endif

check-semver-included:
ifeq ($(SEMVER), 0.0.0)
>	$(error SEMVER is undefined; ex. SEMVER=0.0.1)
endif
