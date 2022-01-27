package pipeline

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/proto"
	"github.com/fatih/color"
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
	CmdPipeline.AddCommand(cmdPipelineList)
}

func pipelineList(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving pipelines")

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
		triggerList := []string{}
		for _, trigger := range pipeline.Triggers {
			triggerStr := fmt.Sprintf("%s(%s)", trigger.Label, color.YellowString(trigger.Kind))
			if trigger.State == proto.PipelineTriggerConfig_DISABLED {
				c := color.New(color.Faint)
				triggerStr = c.Sprintf("%s", triggerStr)
			}
			triggerList = append(triggerList, triggerStr)
		}

		recentRunIDs := getLastNIDs(5, pipeline.LastRunId)
		recentRuns, _ := recentRuns(client, pipeline.Id, recentRunIDs)
		recentRunsHealth := []string{}
		for _, run := range recentRuns {
			recentRunsHealth = append(recentRunsHealth, run.State.String())
		}

		data = append(data, []string{
			pipeline.Id,
			pipeline.Name,
			cliformat.PipelineState(pipeline.State.String()),
			cliformat.Health(recentRunsHealth, false),
			cliformat.UnixMilli(pipeline.Created, "Never", cl.State.Config.Detail),
			cliformat.UnixMilli(pipeline.LastRunTime, "None", cl.State.Config.Detail),
			cliformat.SliceJoin(triggerList, "None"),
		})
	}

	table := formatTable(data, !cl.State.Config.NoColor)

	cl.State.Fmt.Println(table)
	cl.State.Fmt.Finish()

	return nil
}

func formatTable(data [][]string, color bool) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	table.SetHeader([]string{"ID", "Name", "State", "Health", "Created", "Last Run", "Triggers"})
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
