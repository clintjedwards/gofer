package run

import (
	"context"
	"fmt"
	"strconv"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/fatih/color"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunList = &cobra.Command{
	Use:   "list <pipeline_id>",
	Short: "List all runs",
	Long: `List all runs.

A short listing of all pipeline runs.`,
	Example: `$ gofer run list simple`,
	RunE:    runList,
	Args:    cobra.ExactArgs(1),
}

func init() {
	cmdRunList.Flags().IntP("limit", "l", 10, "limit the amount of results returned")
	CmdRun.AddCommand(cmdRunList)
}

func runList(cmd *cobra.Command, args []string) error {
	pipelineID := args[0]

	cl.State.Fmt.Print("Retrieving runs")

	limit, err := cmd.Flags().GetInt("limit")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

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
		Limit:       int64(limit),
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
			cliformat.ColorizeRunState(cliformat.NormalizeEnumValue(run.State.String(), "Unknown")),
			cliformat.ColorizeRunStatus(cliformat.NormalizeEnumValue(run.Status.String(), "Unknown")),
			fmt.Sprintf("%s(%s)", run.Extension.Name, color.YellowString(run.Extension.Label)),
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

	table.SetHeader([]string{"ID", "Started", "Ended", "Duration", "State", "Status", "Triggered By"})
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
			tablewriter.Color(tablewriter.FgBlueColor),
		)
		table.SetColumnColor(
			tablewriter.Color(tablewriter.FgYellowColor),
			tablewriter.Color(0),
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
