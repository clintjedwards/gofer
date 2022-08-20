package pipelines

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/clintjedwards/polyfmt"
	"github.com/fatih/color"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelinesUpdate = &cobra.Command{
	Use:   "update <id> <path>",
	Short: "Update pipeline",
	Long:  `Update pipeline via pipeline configuration.`,
	Example: `$ gofer pipeline update simple ./example_pipelines/rust/simple
$ gofer pipeline update simple ./example_pipelines/go/simple`,
	RunE: pipelinesUpdate,
	Args: cobra.ExactArgs(2),
}

func init() {
	cmdPipelinesUpdate.Flags().BoolP("force", "f", false, "Stop all runs and update pipeline immediately")
	cmdPipelinesUpdate.Flags().BoolP("graceful-stop", "g", false,
		"Stop all runs gracefully; sends a SIGTERM to all task runs for all in-progress runs and then waits for them to stop.")
	CmdPipelines.AddCommand(cmdPipelinesUpdate)
}

func pipelinesUpdate(cmd *cobra.Command, args []string) error {
	id := args[0]
	path := args[1]
	force, err := cmd.Flags().GetBool("force")
	if err != nil {
		fmt.Println(err)
		return err
	}

	gracefully, err := cmd.Flags().GetBool("graceful-stop")
	if err != nil {
		fmt.Println(err)
		return err
	}

	cl.State.Fmt.Print("Updating pipeline")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	cl.State.Fmt.Print("Disabling pipeline")

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.DisablePipeline(ctx, &proto.DisablePipelineRequest{
		NamespaceId: cl.State.Config.Namespace,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not disable pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Disabled pipeline")

	if force {
		cl.State.Fmt.Print("Force cancelling pipeline runs")

		_, err = client.CancelAllRuns(ctx, &proto.CancelAllRunsRequest{PipelineId: id, Force: true})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not force cancel pipeline: %v", err))
			cl.State.Fmt.Finish()
			return err
		}
	}

	if gracefully {
		cl.State.Fmt.Print("Cancelling pipeline runs")

		_, err = client.CancelAllRuns(ctx, &proto.CancelAllRunsRequest{PipelineId: id, Force: false})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not gracefully cancel pipeline: %v", err))
			cl.State.Fmt.Finish()
			return err
		}
	}

	cl.State.Fmt.Print("Waiting for in-progress runs to stop")
	for {
		resp, err := client.ListRuns(ctx, &proto.ListRunsRequest{PipelineId: id, Offset: 0, Limit: 20})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not get run list for pipeline: %v", err))
			time.Sleep(time.Second * 3)
			continue
		}

		hasRunningJob := false
		for _, run := range resp.Runs {
			if run.State != proto.Run_COMPLETE {
				hasRunningJob = true
			}
		}

		if hasRunningJob {
			time.Sleep(time.Second * 5)
			continue
		}

		break
	}

	cl.State.Fmt.PrintSuccess("Checked for runs in-progress")

	cl.State.Fmt.Print("Updating pipeline")

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

	var buildCmd *exec.Cmd

	switch language {
	case configLanguageGolang:
		buildCmd = golangBuildCmd(absolutePath)
	case configLanguageRust:
		buildCmd = rustBuildCmd(absolutePath)
	}

	stderr, err := buildCmd.StderrPipe()
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not read output from cmd; %v", err))
		cl.State.Fmt.Finish()
		return err
	}
	stdout, err := buildCmd.StdoutPipe()
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not read output from cmd; %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	merged := io.MultiReader(stderr, stdout)
	scanner := bufio.NewScanner(merged)
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
			cl.State.Fmt.Print(fmt.Sprintf("Building config: %s", line), polyfmt.Pretty)
		}
		close(lines)
	}()

	err = buildCmd.Run()
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

	linesList := []string{}

	for line := range lines {
		linesList = append(linesList, line)
	}

	if len(linesList) == 0 {
		cl.State.Fmt.PrintErr("No lines found in output")
		cl.State.Fmt.Finish()
		return err
	}

	lastLine := linesList[len(linesList)-1]

	pipelineConfig := proto.PipelineConfig{}

	err = json.Unmarshal([]byte(lastLine), &pipelineConfig)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("Could not parse pipeline config; %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	resp, err := client.UpdatePipeline(ctx, &proto.UpdatePipelineRequest{
		NamespaceId:    cl.State.Config.Namespace,
		PipelineConfig: &pipelineConfig,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not update pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Updated pipeline: [%s] %q", resp.Pipeline.Id, resp.Pipeline.Name))

	cl.State.Fmt.Print("Enabling pipeline")

	_, err = client.EnablePipeline(ctx, &proto.EnablePipelineRequest{
		Id: id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not enable pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Enabled pipeline")
	cl.State.Fmt.Finish()

	return nil
}
