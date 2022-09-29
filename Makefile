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
> CGO_ENABLED=1 go build -ldflags $(GO_LDFLAGS) -o $(OUTPUT)
.PHONY: build

## build-protos: build protobufs
build-protos:
> protoc --proto_path=proto --go_out=proto/go --go_opt=paths=source_relative \
	 --go-grpc_out=proto/go --go-grpc_opt=paths=source_relative proto/*.proto
.PHONY: build-protos

## run: build application and run server
run:
> DEBUG=true; SEMVER=0.0.0; go build -ldflags $(GO_LDFLAGS) -o /tmp/${APP_NAME}
> /tmp/${APP_NAME} service start
.PHONY: run

## run-race: build application and run server with race detector
run-race:
> DEBUG=true; SEMVER=0.0.0; go build -race -ldflags $(GO_LDFLAGS) -o /tmp/${APP_NAME}
> /tmp/${APP_NAME} service start
.PHONY: run-race

## run-website: build website js and run dev server
run-website:
> npm --prefix ./website start
.PHONY: run-website

## build-website: build website js for production
build-website:
> npm --prefix ./website run build
.PHONY: build-website

## deploy-website: build website js and deploy to github pages
deploy-website:
> USE_SSH=true; npm --prefix ./website run build
> USE_SSH_true; npm --prefix ./website run deploy
.PHONY: deploy-website

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
