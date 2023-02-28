package cli

import (
	"bufio"
	"bytes"
	"context"
	"fmt"
	"io"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/clintjedwards/polyfmt"
	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
	pb "google.golang.org/protobuf/proto"
)

type configLanguage string

const (
	configLanguageRust   configLanguage = "RUST"
	configLanguageGolang configLanguage = "GOLANG"
)

var cmdUp = &cobra.Command{
	Use:   "up <path>",
	Short: "Register and deploy a new pipeline config",
	Long: `Register a new pipeline configuration and deploy it.

If pipeline does not exist this will create a new one.

Requires a pipeline configuration file. You can find documentation on how to
create/manage your pipeline configuration file
[here](https://clintjedwards.com/gofer/ref/pipeline_configuration/index.html).`,
	Example: `$ gofer up
$ gofer up ./example_pipelines/rust/simple
$ gofer up ./example_pipelines/go/simple`,
	RunE: pipelineRegister,
}

func init() {
	cmdUp.Flags().BoolP("deploy", "d", true, "performs a registration and deployment of the pipeline."+
		" If this is set to false, the pipeline configuration will be registered but not deployed.")
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

	files, err := ioutil.ReadDir(path)
	if err != nil {
		return "", fmt.Errorf("could not list files in directory")
	}

	for _, file := range files {
		if file.IsDir() {
			continue
		}

		switch strings.ToLower(file.Name()) {
		case "cargo.toml":
			lang = configLanguageRust
		case "go.mod":
			lang = configLanguageGolang
		}
	}

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

func pipelineRegister(cmd *cobra.Command, args []string) error {
	path := "."

	// If the user entered a path, default to that instead.
	if len(args) > 0 {
		path = args[0]
	}

	cl.State.Fmt.Print("Registering pipeline")

	deploy, err := cmd.Flags().GetBool("deploy")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	absolutePath, err := filepath.Abs(path)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not find absolute path for given path %q; %v", path, err))
		cl.State.Fmt.Finish()
		return err
	}

	language, err := detectLanguage(absolutePath)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not determine language for path %q; %v", path, err))
		cl.State.Fmt.Finish()
		return err
	}

	var execCmd *exec.Cmd

	switch language {
	case configLanguageGolang:
		execCmd = golangBuildCmd(absolutePath)
	case configLanguageRust:
		execCmd = rustBuildCmd(absolutePath)
	}

	stderr, err := execCmd.StderrPipe()
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not read output from cmd; %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	outputBuffer := bytes.NewBuffer([]byte{})
	execCmd.Stdout = outputBuffer

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

	err = execCmd.Run()
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

	pipelineConfig := proto.UserPipelineConfig{}

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

	config, err := client.RegisterPipelineConfig(ctx, &proto.RegisterPipelineConfigRequest{
		NamespaceId:    cl.State.Config.Namespace,
		PipelineConfig: &pipelineConfig,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not register pipeline configuration: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("%s: [%s] %q %s", "Registered pipeline", color.BlueString(config.Pipeline.Metadata.Id),
		config.Pipeline.Config.Name, color.MagentaString("v%d", config.Pipeline.Config.Version)))

	if deploy {
		_, err := client.DeployPipeline(ctx, &proto.DeployPipelineRequest{
			NamespaceId: cl.State.Config.Namespace,
			Id:          config.Pipeline.Metadata.Id,
			Version:     config.Pipeline.Config.Version,
			Force:       false,
		})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not deploy pipeline: %v", err))
			cl.State.Fmt.Finish()
			return err
		}
	}

	cl.State.Fmt.Println(fmt.Sprintf("\n  View details of your pipeline: %s",
		color.YellowString("gofer pipeline get %s", config.Pipeline.Metadata.Id)))
	cl.State.Fmt.Println(fmt.Sprintf("  Start a new run: %s", color.YellowString("gofer run start %s", config.Pipeline.Metadata.Id)))

	if config.Pipeline.Config.Version == 1 {
		cl.State.Fmt.Println(fmt.Sprintf("  Subscribe to a extension: %s", color.YellowString("gofer pipeline extension sub %s <extension_name> <extension_label>", config.Pipeline.Metadata.Id)))
	}
	cl.State.Fmt.Finish()

	return nil
}
