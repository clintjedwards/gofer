package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("images", "Images Example Pipeline").
		Description(`This pipeline shows the ways Gofer can build a task's image. `+"`gofer up`"+` builds them on your
machine, uploads them to Gofer, and ties them to the version it registers, so a version always runs exactly what it was
registered with.

Each task starts from a base image. It can copy files in from next to this config and run commands, which happen in
the order they're written, and it can keep only some of what it built on a smaller base. Anything those can't do can
be a Dockerfile instead.

Gofer only builds a task again when its files or how it's built changed, and it keeps the Go and Rust compilers'
caches between builds on its own.`).
		Tasks(
			// Compile a folder, then keep only the binary on a small base. The compiler isn't in the final image.
			sdk.NewTask("report", "golang:1").
				Description("Compiles a small Go program and runs it").
				Copy("./report", "/src").
				Run("cd /src && go build -o /app/report .").
				Keep([]string{"/app/report"}, "debian:13-slim").
				Command("/app/report"),

			// Copy a script into an image that can run it.
			sdk.NewTask("cleanup", "python:3-slim").
				Description("Runs a Python script that's copied in").
				Copy("./cleanup.py", "/cleanup.py").
				Command("python", "/cleanup.py"),

			// A script written right here. It still gets an image of its own.
			sdk.NewTask("notify", "alpine:3.22").
				Description("Runs once the others have finished, whether they worked or not").
				DependsOn("report", sdk.RequiredParentStatusAny).
				DependsOn("cleanup", sdk.RequiredParentStatusAny).
				DependsOn("scan", sdk.RequiredParentStatusAny).
				Script("echo notify: everything else finished"),

			// Anything fancier is a Dockerfile. The last argument is the folder it builds from.
			sdk.NewDockerfileTask("scan", "./scan/Dockerfile", "./scan").
				Description("Builds from a Dockerfile, the same as `docker build ./scan` would"),
		).
		Finish()
	if err != nil {
		log.Fatal(err)
	}
}
