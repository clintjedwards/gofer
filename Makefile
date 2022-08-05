APP_NAME = gofer
GIT_COMMIT = $(shell git rev-parse --short HEAD)
# Although go 1.18 has the git info baked into the binary now it still seems like there is no support
# For including outside variables except this. So keep it for now.
GO_LDFLAGS = '-X "github.com/clintjedwards/${APP_NAME}/internal/cli.appVersion=$(VERSION)" \
				-X "github.com/clintjedwards/${APP_NAME}/internal/api.appVersion=$(VERSION)"'
SHELL = /bin/bash
SEMVER = 0.0.1
VERSION = ${SEMVER}_${GIT_COMMIT}

## build: run tests and compile application
build: check-path-included check-semver-included build-protos
	go test ./... -race
	go mod tidy
	CGO_ENABLED=0 go build -ldflags $(GO_LDFLAGS) -o $(OUTPUT)

## build-protos: build protobufs
build-protos:
	protoc --proto_path=proto --go_out=proto/go --go_opt=paths=source_relative \
	 --go-grpc_out=proto/go --go-grpc_opt=paths=source_relative \
	 proto/*.proto

## run: build application and run server
run: export DEBUG=true
run:
	go build -race -ldflags $(GO_LDFLAGS) -o /tmp/${APP_NAME} && /tmp/${APP_NAME} service start

## run-website: build website js and run dev server
run-website:
	npm --prefix ./website start

## build-website: build website js for production
build-website:
	npm --prefix ./website run build

## deploy-website: build website js and deploy to github pages
deploy-website: export USE_SSH=true
deploy-website:
	npm --prefix ./website run build
	npm --prefix ./website run deploy

## help: prints this help message
help:
	@echo "Usage: "
	@sed -n 's/^##//p' ${MAKEFILE_LIST} | column -t -s ':' |  sed -e 's/^/ /'

check-path-included:
ifndef OUTPUT
	$(error OUTPUT is undefined; ex. OUTPUT=/tmp/${APP_NAME})
endif

check-semver-included:
ifndef SEMVER
	$(error SEMVER is undefined; ex. SEMVER=0.0.1)
endif

