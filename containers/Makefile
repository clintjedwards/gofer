# 	docker build -f extensions/github/Dockerfile -t ghcr.io/clintjedwards/gofer-containers/extensions/github:${semver} .
#	docker tag ghcr.io/clintjedwards/gofer-containers/extensions/github:${semver} ghcr.io/clintjedwards/gofer-containers/extensions/github:latest
#	docker push ghcr.io/clintjedwards/gofer-containers/extensions/github:${semver}
#	docker push ghcr.io/clintjedwards/gofer-containers/extensions/github:latest

## build: build docker containers
build: check-semver-included
	docker build -f extensions/cron/Dockerfile -t ghcr.io/clintjedwards/gofer-containers/extensions/cron:${semver} .
	docker tag ghcr.io/clintjedwards/gofer-containers/extensions/cron:${semver} ghcr.io/clintjedwards/gofer-containers/extensions/cron:latest
	docker build -f extensions/interval/Dockerfile -t ghcr.io/clintjedwards/gofer-containers/extensions/interval:${semver} .
	docker tag ghcr.io/clintjedwards/gofer-containers/extensions/interval:${semver} ghcr.io/clintjedwards/gofer-containers/extensions/interval:latest

	docker build -f debug/envs/Dockerfile -t ghcr.io/clintjedwards/gofer-containers/debug/envs:${semver} .
	docker tag ghcr.io/clintjedwards/gofer-containers/debug/envs:${semver} ghcr.io/clintjedwards/gofer-containers/debug/envs:latest
	docker build -f debug/fail/Dockerfile -t ghcr.io/clintjedwards/gofer-containers/debug/fail:${semver} .
	docker tag ghcr.io/clintjedwards/gofer-containers/debug/fail:${semver} ghcr.io/clintjedwards/gofer-containers/debug/fail:latest
	docker build -f debug/log/Dockerfile -t ghcr.io/clintjedwards/gofer-containers/debug/log:${semver} .
	docker tag ghcr.io/clintjedwards/gofer-containers/debug/log:${semver} ghcr.io/clintjedwards/gofer-containers/debug/log:latest
	docker build -f debug/wait/Dockerfile -t ghcr.io/clintjedwards/gofer-containers/debug/wait:${semver} .
	docker tag ghcr.io/clintjedwards/gofer-containers/debug/wait:${semver} ghcr.io/clintjedwards/gofer-containers/debug/wait:latest

	docker build -f tasks/debug/Dockerfile -t ghcr.io/clintjedwards/gofer-containers/tasks/debug:${semver} .
	docker tag ghcr.io/clintjedwards/gofer-containers/tasks/debug:${semver} ghcr.io/clintjedwards/gofer-containers/tasks/debug:latest

## push: push docker to github
push: check-semver-included
	docker push ghcr.io/clintjedwards/gofer-containers/extensions/cron:${semver}
	docker push ghcr.io/clintjedwards/gofer-containers/extensions/cron:latest
	docker push ghcr.io/clintjedwards/gofer-containers/extensions/interval:${semver}
	docker push ghcr.io/clintjedwards/gofer-containers/extensions/interval:latest

	docker push ghcr.io/clintjedwards/gofer-containers/debug/envs:${semver}
	docker push ghcr.io/clintjedwards/gofer-containers/debug/envs:latest
	docker push ghcr.io/clintjedwards/gofer-containers/debug/fail:${semver}
	docker push ghcr.io/clintjedwards/gofer-containers/debug/fail:latest
	docker push ghcr.io/clintjedwards/gofer-containers/debug/log:${semver}
	docker push ghcr.io/clintjedwards/gofer-containers/debug/log:latest
	docker push ghcr.io/clintjedwards/gofer-containers/debug/wait:${semver}
	docker push ghcr.io/clintjedwards/gofer-containers/debug/wait:latest

	docker push ghcr.io/clintjedwards/gofer-containers/tasks/debug:${semver}
	docker push ghcr.io/clintjedwards/gofer-containers/tasks/debug:latest

## help: prints this help message
help:
	@echo "Usage: "
	@sed -n 's/^##//p' ${MAKEFILE_LIST} | column -t -s ':' |  sed -e 's/^/ /'

check-semver-included:
ifndef semver
	$(error semver is undefined; ex. semver=0.0.1)
endif

