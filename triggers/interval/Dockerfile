FROM golang:1.17 AS build-environment

ARG GOARCH
ARG SEMVER

ADD . /files
WORKDIR /files/triggers/interval

ENV GOOS=linux GOARCH=${GOARCH} CGO_ENABLED=0

RUN go build -o /build/trigger

FROM gcr.io/distroless/static
WORKDIR /app
COPY --from=build-environment /build /app
ENTRYPOINT [ "./trigger" ]
