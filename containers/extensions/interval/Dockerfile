FROM golang:1.22 AS build-environment

LABEL org.opencontainers.image.source https://github.com/clintjedwards/gofer

ARG GOARCH
ARG SEMVER

ADD . /files
WORKDIR /files/extensions/interval

ENV GOOS=linux GOARCH=${GOARCH} CGO_ENABLED=0

RUN go build -o /build/extension

FROM gcr.io/distroless/static:debug
WORKDIR /app
COPY --from=build-environment /build /app
ENTRYPOINT [ "./extension" ]
