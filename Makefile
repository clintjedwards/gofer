## build-protos: build protobufs
build-protos:
	protoc --proto_path=gofer_proto --go_out=gofer_sdk/go/proto --go_opt=paths=source_relative \
	 --go-grpc_out=gofer_sdk/go/proto --go-grpc_opt=paths=source_relative \
	 gofer_proto/*.proto

test:
	cargo test
	cd ./gofer_sdk/go && go test ./...

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
