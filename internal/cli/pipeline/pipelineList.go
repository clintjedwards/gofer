package pipeline

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliFmt "github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineList = &cobra.Command{
	Use:   "list",
	Short: "List all pipelines",
	Long: `List all pipelines.

A short listing of all currently registered pipelines.

Health shows a quick glimpse into how the last 5 builds performed.
  * Unstable = There is a failure in the last 5 builds.
  * Poor = Past 5 builds have all failed.
  * Good = Past 5 builds have all passed.
`,
	Example: `$ gofer pipeline list`,
	RunE:    pipelineList,
}

func init() {
	cmdPipelineList.Flags().IntP("limit", "l", 10, "limit the amount of results returned")
	CmdPipeline.AddCommand(cmdPipelineList)
}

func pipelineList(cmd *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving pipelines")

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

	resp, err := client.ListPipelines(ctx, &proto.ListPipelinesRequest{
		NamespaceId: cl.State.Config.Namespace,
		Limit:       int64(limit),
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list pipelines: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	if len(resp.Pipelines) == 0 {
		cl.State.Fmt.Println("No pipelines found")
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, pipeline := range resp.Pipelines {
		recentRuns := recentRuns(ctx, client, pipeline.Namespace, pipeline.Id, 5)
		recentRunsHealth := []proto.Run_RunStatus{}
		for _, run := range recentRuns {
			recentRunsHealth = append(recentRunsHealth, run.Status)
		}

		var lastRunTime int64
		if len(recentRuns) != 0 {
			lastRun := recentRuns[len(recentRuns)-1]
			lastRunTime = lastRun.Ended
		}

		data = append(data, []string{
			pipeline.Id,
			cliFmt.ColorizePipelineMetadataState(cliFmt.NormalizeEnumValue(pipeline.State.String(), "Unknown")),
			cliFmt.Health(recentRunsHealth, false),
			cliFmt.UnixMilli(pipeline.Created, "Never", cl.State.Config.Detail),
			cliFmt.UnixMilli(lastRunTime, "None", cl.State.Config.Detail),
		})
	}

	table := formatTable(data, !cl.State.Config.NoColor)

	cl.State.Fmt.Println(table)
	cl.State.Fmt.Finish()

	return nil
}

func recentRuns(ctx context.Context, client proto.GoferClient, namespace, pipeline string, limit int64) []*proto.Run {
	resp, err := client.ListRuns(ctx, &proto.ListRunsRequest{
		Offset:      0,
		Limit:       limit,
		NamespaceId: namespace,
		PipelineId:  pipeline,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get recent runs: %v", err))
		return nil
	}

	return resp.Runs
}

func formatTable(data [][]string, color bool) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	table.SetHeader([]string{"ID", "State", "Health", "Created", "Last Run"})
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
		)
		table.SetColumnColor(
			tablewriter.Color(tablewriter.FgYellowColor),
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
