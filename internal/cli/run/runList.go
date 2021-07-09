package run

import (
	"context"
	"fmt"
	"strconv"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/proto"
	"github.com/fatih/color"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunList = &cobra.Command{
	Use:   "list <pipeline_id>",
	Short: "List all runs",
	Long: `List all runs.

A short listing of all currently started runs.`,
	Example: `$ gofer run list simple_test_pipeline`,
	RunE:    runList,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdRun.AddCommand(cmdRunList)
}

func runList(_ *cobra.Command, args []string) error {
	pipelineID := args[0]

	cl.State.Fmt.Print("Retrieving runs")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)

	resp, err := client.ListRuns(ctx, &proto.ListRunsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list runs: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	if len(resp.Runs) == 0 {
		cl.State.Fmt.Println(fmt.Sprintf("No runs found for pipeline %s", pipelineID))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, run := range resp.Runs {
		id := strconv.Itoa(int(run.Id))

		data = append(data, []string{
			id,
			cliformat.UnixMilli(run.Started, "Not yet", cl.State.Config.Detail),
			cliformat.UnixMilli(run.Ended, "Still running", cl.State.Config.Detail),
			cliformat.Duration(run.Started, run.Ended),
			cliformat.RunState(run.State.String()),
			fmt.Sprintf("%s(%s)", run.TriggerName, color.YellowString(run.TriggerKind)),
		})
	}

	table := formatTable(data, !cl.State.Config.NoColor)

	cl.State.Fmt.Println(fmt.Sprintf("  Runs for pipeline %s\n\n%s", color.BlueString(pipelineID), table))
	cl.State.Fmt.Finish()

	return nil
}

func formatTable(data [][]string, color bool) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	table.SetHeader([]string{"ID", "Started", "Ended", "Duration", "Result", "Triggered By"})
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
