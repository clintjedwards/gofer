package config

import (
	"fmt"
	"math"
	"strings"
	"time"
)

// TaskWrapper type simply exists so that we can make structs with fields like "id"
// and we can still add functions called "id()". This makes it not only easier to
// reason about when working with the struct, but when just writing pipelines as an end user.
type TaskWrapper struct {
	Task

	// Builder methods that were used where they can't work, like Copy on a Dockerfile task. They can't return an
	// error without breaking the chain, so Validate reports the first one.
	misused []error
}

// Gofer builds every task's image itself, when you run `gofer up`. A task starts from a base image, copies in your
// files and runs commands, or it points at a Dockerfile.
//
// Example:
//
//	// Compile a folder, then keep only the binary on a small base.
//	NewTask("report", "golang:1").
//		Copy("./report", "/src").
//		Run("cd /src && go build -o /app/report .").
//		Keep([]string{"/app/report"}, "debian:13-slim").
//		Command("/app/report")
//
//	// Anything the steps can't do is a Dockerfile. The last argument is the folder it builds from.
//	NewDockerfileTask("scan", "./scan/Dockerfile", "./scan")
type Task struct {
	ID          string `json:"id"`
	Description string `json:"description"`

	// How the task's image is made.
	Build Build `json:"build"`

	// The image `gofer up` built. A config straight from the SDK doesn't have one; `gofer up` fills it in before it
	// registers the config.
	Image *BuiltImage `json:"image"`

	DependsOn map[string]RequiredParentStatus `json:"depends_on"`
	Variables map[string]string               `json:"variables"`

	// Replaces the image's entrypoint. It's built into the image.
	Entrypoint *[]string `json:"entrypoint"`

	// Replaces the image's command. It's built into the image.
	Command *[]string `json:"command"`

	// Allows users to tell gofer to auto-create and inject API Token into task. If this setting is found, Gofer creates
	// an API key for the run (stored in the user's secret store) and then injects it for this run under the
	// environment variable "GOFER_TOKEN". This key is automatically cleaned up when Gofer attempts to clean up
	// the Run's objects.
	InjectAPIToken bool `json:"inject_api_token"`
	// How long the task's container can run before Gofer stops it and fails the task. 0 means the server's max task
	// execution timeout applies.
	TimeoutSeconds uint64 `json:"timeout_seconds"`
}

// Build says how a task's image is made: from steps, or from a Dockerfile. Exactly one of them is set. The JSON is the
// same as the Rust SDK's, which the server reads.
type Build struct {
	// Start from a base image, then copy files in and run commands, in the order they were written.
	Steps *Steps `json:"steps,omitempty"`

	// Build from a Dockerfile, for anything the steps can't do.
	Dockerfile *Dockerfile `json:"dockerfile,omitempty"`
}

type Steps struct {
	// The image to start from, like "golang:1".
	Base  string `json:"base"`
	Steps []Step `json:"steps"`

	// Moves what it lists onto another base image once every step has run, leaving everything else behind.
	Keep *Keep `json:"keep"`
}

// Step is one step of a build. Exactly one of them is set.
type Step struct {
	Copy *CopyStep `json:"copy,omitempty"`
	Run  *RunStep  `json:"run,omitempty"`
}

// CopyStep copies a file, or what's inside a folder, into the image. From is relative to the folder of the pipeline's
// config, and leaving it is fine, like "../shared".
type CopyStep struct {
	From string `json:"from"`
	To   string `json:"to"`
}

// RunStep runs a command with /bin/sh -c.
type RunStep struct {
	Command string `json:"command"`
}

type Keep struct {
	// Full paths in the image. Each lands at the same path on the new base.
	Paths []string `json:"paths"`

	// The image to move them onto, like "debian:13-slim". It needs whatever the kept files need to run, like the
	// system libraries a program uses.
	Base string `json:"base"`
}

// Dockerfile builds an image from a Dockerfile. Both paths are relative to the folder of the pipeline's config.
type Dockerfile struct {
	Path string `json:"path"`

	// The folder whose files the Dockerfile can use.
	Context string `json:"context"`
}

// BuiltImage is the image `gofer up` built for a task.
type BuiltImage struct {
	// The digest of the image's manifest, like "sha256:4a5c...".
	Digest string `json:"digest"`

	// A hash of the files that went into the image. It's how `gofer up` knows whether they changed since.
	SourceHash string `json:"source_hash"`
}

// NewTask makes a task whose image starts from base, like "golang:1" or "python:3". Add to it with Copy, Run and Keep,
// or leave it as is to run the base image with a command of your own.
func NewTask(id, base string) *TaskWrapper {
	return newTask(id, Build{Steps: &Steps{Base: base, Steps: []Step{}}})
}

// NewDockerfileTask makes a task whose image is built from a Dockerfile, for anything Copy and Run can't do. Both paths
// are relative to the folder of the pipeline's config. The context is the folder the Dockerfile can use files from,
// the same as the one you'd pass to `docker build`, and its .dockerignore is used the same way.
func NewDockerfileTask(id, path, context string) *TaskWrapper {
	return newTask(id, Build{Dockerfile: &Dockerfile{Path: path, Context: context}})
}

func newTask(id string, build Build) *TaskWrapper {
	return &TaskWrapper{
		Task: Task{
			ID:             id,
			Description:    "",
			Build:          build,
			DependsOn:      make(map[string]RequiredParentStatus),
			Variables:      make(map[string]string),
			InjectAPIToken: false,
		},
	}
}

// Copy copies a file, or what's inside a folder, into the image at to. From is relative to the folder of the pipeline's
// config, and leaving it is fine, like "../shared".
//
// The folder's .dockerignore decides what to leave out. Folders that only hold a cache are left out too, so copying a
// project doesn't copy what a local build left behind.
func (t *TaskWrapper) Copy(from, to string) *TaskWrapper {
	return t.addStep("copy", from, Step{Copy: &CopyStep{From: from, To: to}})
}

// Run runs a command in the image with /bin/sh -c, after the steps before it. Gofer keeps caches for Go and Cargo
// between builds on its own, so a plain `go build` is only slow the first time.
func (t *TaskWrapper) Run(command string) *TaskWrapper {
	return t.addStep("run", command, Step{Run: &RunStep{Command: command}})
}

// Keep moves only paths onto base once every step has run, and leaves everything else behind, including the compiler.
// A smaller image uploads and starts faster. Paths have to be full paths, and each one lands at the same path on the
// new base.
//
// Nothing else carries over from the first base: no environment variables, working directory or entrypoint. The new
// base needs whatever the kept files need to run. Calling it again replaces what was kept before.
func (t *TaskWrapper) Keep(paths []string, base string) *TaskWrapper {
	if t.Task.Build.Steps == nil {
		t.misuse("keep", strings.Join(paths, ", "))
		return t
	}

	t.Task.Build.Steps.Keep = &Keep{Paths: paths, Base: base}
	return t
}

func (t *TaskWrapper) addStep(name, value string, step Step) *TaskWrapper {
	if t.Task.Build.Steps == nil {
		t.misuse(name, value)
		return t
	}

	t.Task.Build.Steps.Steps = append(t.Task.Build.Steps.Steps, step)
	return t
}

func (t *TaskWrapper) misuse(name, value string) {
	t.misused = append(t.misused, fmt.Errorf("invalid %s for task %q: %q; can't be used on a task built from a "+
		"Dockerfile; do it in the Dockerfile instead", name, t.Task.ID, value))
}

// validate mirrors the Rust SDK's checks; the Gofer API runs those on registration either way.
func (t *TaskWrapper) validate() error {
	if err := validateIdentifier("id", t.Task.ID); err != nil {
		return err
	}

	if len(t.misused) > 0 {
		return t.misused[0]
	}

	empty := func(argument, value string) error {
		if strings.TrimSpace(value) == "" {
			return fmt.Errorf("invalid %s for task %q; cannot be empty", argument, t.Task.ID)
		}
		return nil
	}

	build := t.Task.Build
	if (build.Steps == nil) == (build.Dockerfile == nil) {
		return fmt.Errorf("task %q has to be built from either steps or a Dockerfile", t.Task.ID)
	}

	if build.Dockerfile != nil {
		if err := empty("Dockerfile path", build.Dockerfile.Path); err != nil {
			return err
		}
		return empty("Dockerfile context", build.Dockerfile.Context)
	}

	if err := empty("base image", build.Steps.Base); err != nil {
		return err
	}

	for _, step := range build.Steps.Steps {
		switch {
		case step.Copy != nil:
			if err := empty("copy source", step.Copy.From); err != nil {
				return err
			}
			if err := empty("copy destination", step.Copy.To); err != nil {
				return err
			}
		case step.Run != nil:
			if err := empty("run command", step.Run.Command); err != nil {
				return err
			}
		}
	}

	if keep := build.Steps.Keep; keep != nil {
		if err := empty("keep base image", keep.Base); err != nil {
			return err
		}
		if len(keep.Paths) == 0 {
			return fmt.Errorf("invalid keep for task %q; needs at least one path to keep", t.Task.ID)
		}

		// A relative path would be relative to whatever folder each image starts in, which can differ.
		for _, path := range keep.Paths {
			if !strings.HasPrefix(path, "/") {
				return fmt.Errorf("invalid keep path for task %q: %q; has to be a full path, starting with '/'",
					t.Task.ID, path)
			}
		}
	}

	return nil
}

// Add a short description of the task's purpose.
func (t *TaskWrapper) Description(description string) *TaskWrapper {
	t.Task.Description = description
	return t
}

// Add a single task dependency. This allows you to tie a task's execution to the result of another task.
func (t *TaskWrapper) DependsOn(taskID string, state RequiredParentStatus) *TaskWrapper {
	t.Task.DependsOn[taskID] = state
	return t
}

// Add multiple task dependencies. This allows you to tie a task's execution to the result of several other tasks.
func (t *TaskWrapper) DependsOnMany(dependsOn map[string]RequiredParentStatus) *TaskWrapper {
	for id, status := range dependsOn {
		t.Task.DependsOn[id] = status
	}
	return t
}

// Add multiple variables. Variables are passed to your task as environment variables in a key value fashion.
// Variable values can also be pulled from other resources within Gofer like the secret store. Making it easy to
// pass in things like secrets.
func (t *TaskWrapper) Variables(variables map[string]string) *TaskWrapper {
	for key, value := range variables {
		t.Task.Variables[key] = value
	}
	return t
}

// Replaces the image's [entrypoint], the same way `docker run --entrypoint` would, so the image's command goes too
// unless you set one. It's built into the image.
//
// [entrypoint]: https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact
func (t *TaskWrapper) Entrypoint(entrypoint ...string) *TaskWrapper {
	t.Task.Entrypoint = &entrypoint
	return t
}

// Replaces the image's [command]. It's built into the image.
//
// [command]: https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact
func (t *TaskWrapper) Command(command ...string) *TaskWrapper {
	t.Task.Command = &command
	return t
}

// Runs Script's script with bash when the image has it and sh otherwise. The script is passed as $1 rather than
// pasted into this string so it never needs quoting.
const scriptLauncher = `exec "$(command -v bash || echo sh)" -c "$1"`

// Provide an inline multi-line shell script to be run in the container.
//
// The script is run with `bash -c "<script>"` if the image has bash, and `sh -c "<script>"` if it doesn't, so it
// works on small images like alpine. If you use bash features, make sure your image has bash. Leading and trailing
// whitespace is trimmed.
//
// Example:
//
//	sdk.NewTask("run-go-test", "golang:latest").
//		Script(`
//			go test ./...
//			wget https://example.com/somefile
//			curl https://google.com
//		`)
//
// In this example, all three commands (go test, wget, and curl) will run
// sequentially inside a single container session.
//
// Should not be used with [TaskWrapper.Command].
func (t *TaskWrapper) Script(script string) *TaskWrapper {
	trimmedScript := strings.TrimSpace(script)

	t.Task.Command = &[]string{"sh", "-c", scriptLauncher, "gofer-script", trimmedScript}
	return t
}

// Gofer will auto-generate and inject a Gofer API token as `GOFER_TOKEN`. This allows you to easily have tasks
// communicate with Gofer by either embedding Gofer's CLI or just simply using the token to authenticate to the API.
//
// The token belongs to the run, and Gofer deletes it when the run ends.
func (t *TaskWrapper) InjectAPIToken(injectToken bool) *TaskWrapper {
	t.Task.InjectAPIToken = injectToken
	return t
}

// How long the task's container can run before Gofer stops it and marks the task as failed. Time spent loading the
// image doesn't count. Without a timeout, or with one longer than the server allows, the server's max task execution
// timeout applies instead.
//
// Timeouts are in whole seconds; anything shorter is rounded up.
func (t *TaskWrapper) Timeout(timeout time.Duration) *TaskWrapper {
	// Converting a negative float to uint64 isn't defined in Go, so treat it as no timeout.
	if timeout < 0 {
		timeout = 0
	}
	t.Task.TimeoutSeconds = uint64(math.Ceil(timeout.Seconds()))
	return t
}
