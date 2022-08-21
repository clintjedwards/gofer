package token

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/olekukonko/tablewriter"
	"golang.org/x/text/cases"
	"golang.org/x/text/language"

	"github.com/clintjedwards/gofer/internal/cli/format"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTokenList = &cobra.Command{
	Use:   "list",
	Short: "List all tokens for a specific namespace",
	RunE:  tokenList,
}

func init() {
	CmdToken.AddCommand(cmdTokenList)
}

func formatTokenKind(kind string) string {
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)

	kind = strings.ReplaceAll(kind, "_", " ")
	kind = toTitle.String(toLower.String(kind))

	return kind
}

func tokenList(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving tokens")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListTokens(ctx, &proto.ListTokensRequest{
		Namespace: cl.State.Config.Namespace,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, protoToken := range resp.Tokens {
		token := models.Token{}
		token.FromProto(protoToken)

		data = append(data, []string{
			formatTokenKind(string(token.Kind)),
			format.UnixMilli(token.Created, "Unknown", cl.State.Config.Detail),
			format.UnixMilli(token.Expires, "Unknown", cl.State.Config.Detail),
			printMap(token.Metadata),
			fmt.Sprintf("%v", token.Namespaces),
		})
	}

	table := formatTable(data, true)

	cl.State.Fmt.Println(table)
	cl.State.Fmt.Finish()

	return nil
}

func printMap(item map[string]string) string {
	output := ""

	for key, value := range item {
		output += fmt.Sprintf("%s: %s\n", key, value)
	}

	return output
}

func formatTable(data [][]string, color bool) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	table.SetHeader([]string{"Kind", "Created", "Expires", "Metadata", "Namespaces"})
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
