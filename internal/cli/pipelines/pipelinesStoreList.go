package pipelines

import (
	"context"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/olekukonko/tablewriter"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelinesStoreList = &cobra.Command{
	Use:     "list <pipeline_id>",
	Short:   "List all objects from the pipeline specific store",
	Example: `$ gofer pipelines store list simple_test_pipeline 5`,
	RunE:    storeList,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipelinesStore.AddCommand(cmdPipelinesStoreList)
}

func storeList(_ *cobra.Command, args []string) error {
	cl.State.Fmt.Print("Retrieving object keys")
	pipelineID := args[0]

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListPipelineObjects(ctx, &proto.ListPipelineObjectsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, key := range resp.Keys {
		data = append(data, []string{
			key.Key,
			cliformat.UnixMilli(key.Created, "Never", cl.State.Config.Detail),
		})
	}

	table := formatStoreTable(data, !cl.State.Config.NoColor)

	cl.State.Fmt.Println(table)
	cl.State.Fmt.Finish()
	return nil
}

func formatStoreTable(data [][]string, color bool) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	table.SetHeader([]string{"Key", "Created"})
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
		)
		table.SetColumnColor(
			tablewriter.Color(tablewriter.FgYellowColor),
			tablewriter.Color(0),
		)
	}

	table.AppendBulk(data)

	table.Render()
	return tableString.String()
}
