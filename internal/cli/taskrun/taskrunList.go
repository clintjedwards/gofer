package taskrun

import (
	"context"
	"fmt"
	"strconv"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/fatih/color"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTaskRunList = &cobra.Command{
	Use:   "list <pipeline> <run>",
	Short: "List all taskruns",
	Long: `List all taskruns.

A short listing of all task runs for a specific run.`,
	Example: `$ gofer taskrun list simple_test_pipeline 15`,
	RunE:    taskrunList,
	Args:    cobra.ExactArgs(2),
}

func init() {
	CmdTaskRun.AddCommand(cmdTaskRunList)
}

func taskrunList(_ *cobra.Command, args []string) error {
	pipelineID := args[0]
	runIDRaw := args[1]
	runID, err := strconv.Atoi(runIDRaw)
	if err != nil {
		return err
	}

	cl.State.Fmt.Print("Retrieving taskruns")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListTaskRuns(ctx, &proto.ListTaskRunsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		RunId:       int64(runID),
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list taskruns: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, protoTaskrun := range resp.TaskRuns {
		taskrun := models.TaskRun{}
		taskrun.FromProto(protoTaskrun)

		data = append(data, []string{
			taskrun.ID,
			cliformat.UnixMilli(taskrun.Started, "Not yet", cl.State.Config.Detail),
			cliformat.UnixMilli(taskrun.Ended, "Still running", cl.State.Config.Detail),
			cliformat.Duration(taskrun.Started, taskrun.Ended),
			cliformat.ColorizeTaskRunStatus(cliformat.NormalizeEnumValue(taskrun.Status, "Unknown")),
			cliformat.ColorizeTaskRunState(cliformat.NormalizeEnumValue(taskrun.State, "Unknown")),
		})
	}

	table := formatTable(data, !cl.State.Config.NoColor)

	cl.State.Fmt.Println(fmt.Sprintf("  TaskRuns for run %s, pipeline %s\n\n%s",
		color.BlueString("#"+runIDRaw), color.BlueString(pipelineID), table))
	cl.State.Fmt.Finish()

	return nil
}

func formatTable(data [][]string, color bool) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	table.SetHeader([]string{"ID", "Started", "Ended", "Duration", "Status", "State"})
	table.SetAlignment(tablewriter.ALIGN_LEFT)
	table.SetHeaderAlignment(tablewriter.ALIGN_LEFT)
	table.SetHeaderLine(true)
	table.SetBorder(false)
	table.SetAutoFormatHeaders(false)
	table.SetRowSeparator("―")
	table.SetRowLine(false)
	table.SetColumnSeparator("")
	table.SetCenterSeparator("")

	if color {
		table.SetHeaderColor(
			tablewriter.Color(tablewriter.FgBlueColor),
			tablewriter.Color(tablewriter.FgBlueColor),
			tablewriter.Color(tablewriter.FgBlueColor),
			tablewriter.Color(tablewriter.FgBlueColor),
			tablewriter.Color(tablewriter.FgBlueColor),
			tablewriter.Color(tablewriter.FgBlueColor),
		)
		table.SetColumnColor(
			tablewriter.Color(tablewriter.FgYellowColor),
			tablewriter.Color(0),
			tablewriter.Color(0),
			tablewriter.Color(0),
			tablewriter.Color(0),
			tablewriter.Color(0),
		)
	}

	table.AppendBulk(data)

	table.Render()
	return tableString.String()
}
