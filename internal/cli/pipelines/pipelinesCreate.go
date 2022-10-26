package pipelines

import (
	"bufio"
	"bytes"
	"context"
	"fmt"
	"io"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/clintjedwards/polyfmt"
	"google.golang.org/grpc/metadata"
	pb "google.golang.org/protobuf/proto"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

var cmdPipelinesCreate = &cobra.Command{
	Use:   "create <path>",
	Short: "Create a new pipeline",
	Long: `Create a new pipeline.

Updating a pipeline requires a pipeline configuration file. You can find documentation on how to
manage your pipeline configuration file
[here](https://clintjedwards.com/gofer/ref/pipeline_configuration/index.html).`,
	Example: `$ gofer pipeline create ./example_pipelines/rust/simple
$ gofer pipeline create ./example_pipelines/go/simple`,
	RunE: pipelinesCreate,
	Args: cobra.ExactArgs(1),
}

func init() {
	CmdPipelines.AddCommand(cmdPipelinesCreate)
}

func detectLanguage(path string) (configLanguage, error) {
	stat, err := os.Stat(path)
	if err != nil {
		return "", err
	}

	if !stat.IsDir() {
		return "", fmt.Errorf("path must be a directory")
	}

	var lang configLanguage

	_ = filepath.WalkDir(path, func(_ string, d fs.DirEntry, _ error) error {
		info, err := d.Info()
		if err != nil {
			return nil
		}

		if info.IsDir() {
			return nil
		}

		switch info.Name() {
		case "Cargo.toml":
			lang = configLanguageRust
			return fmt.Errorf("found language")
		case "go.mod":
			lang = configLanguageGolang
			return fmt.Errorf("found language")
		}

		return nil
	})

	if lang == "" {
		return lang, fmt.Errorf("no 'Cargo.toml' or 'go.mod' found")
	}

	return lang, nil
}

func golangBuildCmd(path string) *exec.Cmd {
	cmd := exec.Command("/bin/sh", "-c", "go build -v -o /tmp/gofer_go_pipeline && /tmp/gofer_go_pipeline")
	cmd.Dir = path
	return cmd
}

func rustBuildCmd(path string) *exec.Cmd {
	cmd := exec.Command("cargo", "run", fmt.Sprintf("--manifest-path=%s/Cargo.toml", path))
	return cmd
}

func golangCmdString(path string) string {
	return fmt.Sprintf("cd %s && go build -o /tmp/gofer_go_pipeline && /tmp/gofer_go_pipeline", path)
}

func rustCmdString(path string) string {
	return fmt.Sprintf("cargo run --manifest-path %s/Cargo.toml", path)
}

func pipelinesCreate(_ *cobra.Command, args []string) error {
	path := args[0]

	cl.State.Fmt.Print("Creating pipeline")

	absolutePath, err := filepath.Abs(path)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not find absolute path for given path %q; %v", path, err))
		cl.State.Fmt.Finish()
		return err
	}

	language, err := detectLanguage(absolutePath)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not determine language for %q; %v", path, err))
		cl.State.Fmt.Finish()
		return err
	}

	var cmd *exec.Cmd

	switch language {
	case configLanguageGolang:
		cmd = golangBuildCmd(absolutePath)
	case configLanguageRust:
		cmd = rustBuildCmd(absolutePath)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not read output from cmd; %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	outputBuffer := bytes.NewBuffer([]byte{})
	cmd.Stdout = outputBuffer

	// By default the diagnostic output for a program should probably write to stderr.
	// We will assume (maybe naively) this is the case for both the go compiler and the rust
	// compiler(https://github.com/rust-lang/cargo/issues/1473) and make anything printing to
	// stderr go to the user as an update and anything in stdout must be the binary output
	// we're looking for.

	scanner := bufio.NewScanner(stderr)
	lines := make(chan string, 2000)

	go func() {
		for scanner.Scan() {
			line := scanner.Text()
			line = strings.TrimSpace(line)
			lines <- line // Put the line into the lines buffer before truncating it.

			// Truncate the line so that it fits better in small command lines.
			if len(line) > 80 {
				line = line[:80]
			}
			if line == "" {
				line = "compiling"
			}

			cl.State.Fmt.Print(fmt.Sprintf("Building config: %s", line), polyfmt.Pretty)
		}
		close(lines)
	}()

	err = cmd.Run()
	if err != nil {
		cl.State.Fmt.PrintErr("Could not successfully build target pipeline; Examine partial error output below:\n...")

		linesList := []string{}

		for line := range lines {
			linesList = append(linesList, line)
		}

		if len(linesList) == 0 {
			lines <- "No output found for this pipeline build"
		}

		if len(linesList) > 15 {
			linesList = linesList[:15]
		}

		for _, line := range linesList {
			cl.State.Fmt.Println(fmt.Sprintf("  %s", line))
		}

		switch language {
		case configLanguageRust:
			cl.State.Fmt.Println(fmt.Sprintf("...\nView full error output: %s", color.CyanString(rustCmdString(path))))
		case configLanguageGolang:
			cl.State.Fmt.Println(fmt.Sprintf("...\nView full error output: %s", color.CyanString(golangCmdString(path))))
		}
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Print("Parsing pipeline config")

	output, err := io.ReadAll(outputBuffer)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("Could not parse pipeline config; %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	if len(output) == 0 {
		cl.State.Fmt.PrintErr("No lines found in output")
		cl.State.Fmt.Finish()
		return err
	}

	pipelineConfig := proto.PipelineConfig{}

	err = pb.Unmarshal(output, &pipelineConfig)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("Could not parse pipeline config; %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Print("Creating pipeline config")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)

	resp, err := client.CreatePipeline(ctx, &proto.CreatePipelineRequest{
		NamespaceId:    cl.State.Config.Namespace,
		PipelineConfig: &pipelineConfig,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not create pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	printCreateSuccess(resp.Pipeline)

	return nil
}

func printCreateSuccess(pipeline *proto.Pipeline) {
	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Created pipeline: [%s] %q", color.BlueString(pipeline.Id), pipeline.Name))
	cl.State.Fmt.Println(fmt.Sprintf("\n  View details of your new pipeline: %s", color.YellowString("gofer pipelines get %s", pipeline.Id)))
	cl.State.Fmt.Println(fmt.Sprintf("  Start a new run: %s", color.YellowString("gofer runs start %s", pipeline.Id)))
	cl.State.Fmt.Finish()
}
