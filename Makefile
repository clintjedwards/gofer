APP_NAME = gofer
EPOCH_TIME = $(shell date +%s)
GIT_COMMIT = $(shell git rev-parse --short HEAD)
GO_LDFLAGS = '-X "github.com/clintjedwards/${APP_NAME}/internal/cli.appVersion=$(VERSION)" \
				-X "github.com/clintjedwards/${APP_NAME}/internal/api.appVersion=$(VERSION)"'
SHELL = /bin/bash
SEMVER = 0.0.1
VERSION = ${SEMVER}_${GIT_COMMIT}_${EPOCH_TIME}

## build: run tests and compile application
build: check-path-included build-protos
	go test ./...
	go mod tidy
	CGO_ENABLED=0 go build -ldflags $(GO_LDFLAGS) -o $(path)

## build-triggers: build trigger docker containers
build-triggers:
	docker build -f triggers/cron/Dockerfile -t ghcr.io/clintjedwards/gofer/trigger_cron:latest .
	docker build -f triggers/interval/Dockerfile -t ghcr.io/clintjedwards/gofer/trigger_interval:latest .

## push-triggers: push default trigger docker to github
push-triggers:
	docker push ghcr.io/clintjedwards/gofer/trigger_cron:latest
	docker push ghcr.io/clintjedwards/gofer/trigger_interval:latest

## build-protos: build protobufs
build-protos:
	protoc --go_out=. --go_opt=paths=source_relative \
	 --go-grpc_out=. --go-grpc_opt=paths=source_relative \
	 sdk/proto/*.proto
	protoc --proto_path=proto --go_out=proto --go_opt=paths=source_relative \
	 --go-grpc_out=proto --go-grpc_opt=paths=source_relative \
	 proto/*.proto

## run: build application and run server
run: export DEBUG=true
run:
	go build -ldflags $(GO_LDFLAGS) -o /tmp/${APP_NAME} && /tmp/${APP_NAME} service start

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
ifndef path
	$(error path is undefined; ex. path=/tmp/${APP_NAME})
endif

