package secrets

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/olekukonko/tablewriter"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineSecretsList = &cobra.Command{
	Use:     "list <pipeline_id>",
	Short:   "View keys from the pipeline secret store",
	Example: `$ gofer secrets pipeline list simple`,
	RunE:    pipelineSecretsStoreList,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipelineSecrets.AddCommand(cmdPipelineSecretsList)
}

func pipelineSecretsStoreList(_ *cobra.Command, args []string) error {
	id := args[0]

	cl.State.Fmt.Print("Retrieving pipeline keys")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListPipelineSecrets(ctx, &proto.ListPipelineSecretsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list keys: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, key := range resp.Keys {
		data = append(data, []string{
			key.Key,
			format.UnixMilli(key.Created, "Never", cl.State.Config.Detail),
		})
	}

	table := formatPipelineTable(data, !cl.State.Config.NoColor)

	cl.State.Fmt.Println(table)
	cl.State.Fmt.Finish()
	return nil
}

func formatPipelineTable(data [][]string, color bool) string {
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
