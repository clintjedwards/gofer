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
build: check-path-included check-semver-included build-protos
> go test ./... -race
> go mod tidy
> export CGO_ENABLED=1
> go build -ldflags $(GO_LDFLAGS) -o $(OUTPUT)
.PHONY: build

## build-protos: build protobufs
build-protos:
> protoc --proto_path=proto --go_out=proto/go --go_opt=paths=source_relative \
	 --go-grpc_out=proto/go --go-grpc_opt=paths=source_relative proto/*.proto
.PHONY: build-protos

## run: build application and run server
run:
> export GOFER_DEBUG=true
> export GOFER_DEVMODE=true
> export SEMVER=0.0.0
> go build -ldflags $(GO_LDFLAGS) -o /tmp/${APP_NAME}
> /tmp/${APP_NAME} service start
.PHONY: run

## run-race: build application and run server with race detector
run-race:
> export GOFER_DEBUG=true
> export GOFER_DEVMODE=true
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
> git subtree split --prefix documentation/book/html -b gh-pages
> git push -f origin gh-pages:gh-pages
> git branch -D gh-pages
.PHONY: push-docs

# 	docker build -f triggers/github/Dockerfile -t ghcr.io/clintjedwards/gofer/triggers/github:${semver} .
#	docker tag ghcr.io/clintjedwards/gofer/triggers/github:${semver} ghcr.io/clintjedwards/gofer/triggers/github:latest
#	docker push ghcr.io/clintjedwards/gofer/triggers/github:${semver}
#	docker push ghcr.io/clintjedwards/gofer/triggers/github:latest

## build-containers: build docker containers
build-containers: check-semver-included
> cd containers
> docker build -f triggers/cron/Dockerfile -t ghcr.io/clintjedwards/gofer/triggers/cron:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/triggers/cron:${SEMVER} ghcr.io/clintjedwards/gofer/triggers/cron:latest
> docker build -f triggers/interval/Dockerfile -t ghcr.io/clintjedwards/gofer/triggers/interval:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/triggers/interval:${SEMVER} ghcr.io/clintjedwards/gofer/triggers/interval:latest

> docker build -f debug/envs/Dockerfile -t ghcr.io/clintjedwards/gofer/debug/envs:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/debug/envs:${SEMVER} ghcr.io/clintjedwards/gofer/debug/envs:latest
> docker build -f debug/fail/Dockerfile -t ghcr.io/clintjedwards/gofer/debug/fail:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/debug/fail:${SEMVER} ghcr.io/clintjedwards/gofer/debug/fail:latest
> docker build -f debug/log/Dockerfile -t ghcr.io/clintjedwards/gofer/debug/log:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/debug/log:${SEMVER} ghcr.io/clintjedwards/gofer/debug/log:latest
> docker build -f debug/wait/Dockerfile -t ghcr.io/clintjedwards/gofer/debug/wait:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/debug/wait:${SEMVER} ghcr.io/clintjedwards/gofer/debug/wait:latest

> docker build -f tasks/debug/Dockerfile -t ghcr.io/clintjedwards/gofer/tasks/debug:${SEMVER} .
> docker tag ghcr.io/clintjedwards/gofer/tasks/debug:${SEMVER} ghcr.io/clintjedwards/gofer/tasks/debug:latest

## push-containers: push docker containers to github
push-containers: check-semver-included
> docker push ghcr.io/clintjedwards/gofer/triggers/cron:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/triggers/cron:latest
> docker push ghcr.io/clintjedwards/gofer/triggers/interval:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/triggers/interval:latest

> docker push ghcr.io/clintjedwards/gofer/debug/envs:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/debug/envs:latest
> docker push ghcr.io/clintjedwards/gofer/debug/fail:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/debug/fail:latest
> docker push ghcr.io/clintjedwards/gofer/debug/log:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/debug/log:latest
> docker push ghcr.io/clintjedwards/gofer/debug/wait:${SEMVER}
> docker push ghcr.io/clintjedwards/gofer/debug/wait:latest

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
