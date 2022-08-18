package secret

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

var cmdGlobalSecretsList = &cobra.Command{
	Use:     "list",
	Short:   "View keys from the global secret store",
	Example: `$ gofer secrets global list`,
	RunE:    globalSecretsStoreList,
}

func init() {
	CmdGlobalSecrets.AddCommand(cmdGlobalSecretsList)
}

func globalSecretsStoreList(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving global keys")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListGlobalSecrets(ctx, &proto.ListGlobalSecretsRequest{})
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

	table := formatGlobalTable(data, !cl.State.Config.NoColor)

	cl.State.Fmt.Println(table)
	cl.State.Fmt.Finish()
	return nil
}

func formatGlobalTable(data [][]string, color bool) string {
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
