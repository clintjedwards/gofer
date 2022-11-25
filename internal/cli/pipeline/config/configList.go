package config

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

var cmdPipelineConfigList = &cobra.Command{
	Use:     "list <pipeline_id>",
	Short:   "List pipeline configs",
	Long:    `List pipeline configurations.`,
	Example: `$ gofer pipeline config list simple`,
	RunE:    pipelineConfigList,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipelineConfig.AddCommand(cmdPipelineConfigList)
}

func pipelineConfigList(_ *cobra.Command, args []string) error {
	pipelineID := args[0]

	cl.State.Fmt.Print("Retrieving pipeline configs")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListPipelineConfigs(ctx, &proto.ListPipelineConfigsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get pipeline config configs: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	if len(resp.Configs) == 0 {
		cl.State.Fmt.Println(fmt.Sprintf("No pipeline configs found for pipeline %s", pipelineID))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, config := range resp.Configs {
		data = append(data, []string{
			fmt.Sprint(config.Version),
			cliFmt.ColorizePipelineConfigState(cliFmt.NormalizeEnumValue(config.State.String(), "Unknown")),
			cliFmt.UnixMilli(config.Registered, "Never", cl.State.Config.Detail),
			cliFmt.UnixMilli(config.Deprecated, "Never", cl.State.Config.Detail),
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

	table.SetHeader([]string{"Version", "State", "Registered", "Deprecated"})
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
		)
		table.SetColumnColor(
			tablewriter.Color(tablewriter.FgYellowColor),
			tablewriter.Color(0),
			tablewriter.Color(0),
			tablewriter.Color(0),
		)
	}

	table.AppendBulk(data)

	table.Render()
	return tableString.String()
}
