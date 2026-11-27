package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

// Points Go's caches into the run folder, so the tasks share what the first one downloads and builds. GOPROXY=off
// on the tasks after it proves they don't download anything again.
func goCaches(proxy string) map[string]string {
	return map[string]string{
		"GOMODCACHE": "/gofer/run/cache/mod",
		"GOCACHE":    "/gofer/run/cache/build",
		"GOPROXY":    proxy,
	}
}

func main() {
	pipeline := sdk.NewPipeline("run-folder", "Run Folder Example Pipeline").
		Description(`This pipeline shows what tasks can do with the folder they share at /gofer/run. Each run gets an
empty one, every task in the run can read and write it, and Gofer deletes it when the run ends.

It clones Gofer's own repository once, downloads its Go dependencies once, then tests the Go SDK and builds a small
program at the same time from that one checkout and cache. A task running as a different user packages what was
built, and the last task writes a summary and keeps it as a pipeline object, since the folder is gone afterward:

    gofer pipeline object get summary --pipeline run-folder --stringify

To build another branch or tag: gofer pipeline run run-folder -v REF=<branch or tag>`)

	// Clone once, and only the folders the run uses. Every task after this reads the same files.
	pipeline.Task(sdk.NewTask("checkout", "alpine:3.22").
		Description("Clones the repository into the run folder and works out the version").
		Run("apk add --no-cache git").
		Variables(map[string]string{
			"REPO": "https://github.com/clintjedwards/gofer.git",
			"REF":  "main",
		}).
		Script(`
			set -eu
			git clone --quiet --depth 1 --filter=blob:none --sparse --branch "$REF" "$REPO" /gofer/run/src
			git -C /gofer/run/src sparse-checkout set sdk/go examplePipelines/go/simple
			git -C /gofer/run/src rev-parse --short HEAD > /gofer/run/version
			echo "checked out $REF at $(cat /gofer/run/version)"
		`))

	pipeline.Task(sdk.NewTask("download", "golang:1").
		Description("Downloads the Go modules once, for every task after it").
		DependsOn("checkout", sdk.RequiredParentStatusSuccess).
		Variables(goCaches("https://proxy.golang.org,direct")).
		Script(`
			set -eu
			cd /gofer/run/src/sdk/go && go mod download
			cd /gofer/run/src/examplePipelines/go/simple && go mod download
		`))

	// Test and build run at the same time. Each writes to a folder of its own so they don't overwrite each other, and
	// Go's caches are safe to share between them.
	pipeline.Task(sdk.NewTask("test", "golang:1").
		Description("Tests the Go SDK and saves the results for the summary").
		DependsOn("download", sdk.RequiredParentStatusSuccess).
		Variables(goCaches("off")).
		Script(`
			set -eu
			mkdir -p /gofer/run/reports
			cd /gofer/run/src/sdk/go
			go test -json ./... > /gofer/run/reports/tests.json
		`))

	pipeline.Task(sdk.NewTask("build", "golang:1").
		Description("Builds the simple example pipeline for two platforms").
		DependsOn("download", sdk.RequiredParentStatusSuccess).
		Variables(goCaches("off")).
		Script(`
			set -eu
			cd /gofer/run/src/examplePipelines/go/simple
			for arch in amd64 arm64; do
				CGO_ENABLED=0 GOOS=linux GOARCH=$arch go build -o /gofer/run/dist/linux-$arch/simple .
			done
		`))

	// The Dockerfile makes this run as nobody, while the tasks before it ran as root. It can read what they made and
	// write new files to the run folder, but it couldn't change their files.
	pipeline.Task(sdk.NewDockerfileTask("package", "./package/Dockerfile", "./package").
		Description("Packages each build with a checksum, as a user other than root").
		DependsOn("build", sdk.RequiredParentStatusSuccess))

	// Runs even when tests fail or nothing got built, since that's when a summary is most useful. The run folder goes
	// when the run ends, so the summary is kept as a pipeline object, with the gofer CLI every task has.
	pipeline.Task(sdk.NewTask("summary", "python:3-slim").
		Description("Writes a summary of the run and keeps it as a pipeline object").
		DependsOn("test", sdk.RequiredParentStatusAny).
		DependsOn("package", sdk.RequiredParentStatusAny).
		Copy("./summary.py", "/summary.py").
		InjectAPIToken(true).
		Script(`
			set -eu
			python /summary.py
			gofer pipeline object put summary /gofer/run/summary.md --force
			echo "kept as the pipeline object 'summary'; gofer pipeline object get summary --pipeline $GOFER_PIPELINE_ID --stringify"
		`))

	if err := pipeline.Finish(); err != nil {
		log.Fatal(err)
	}
}
