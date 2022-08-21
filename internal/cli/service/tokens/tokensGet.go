package tokens

import (
	"bytes"
	"context"
	"fmt"
	"html/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/fatih/color"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTokensGet = &cobra.Command{
	Use:   "get",
	Short: "Get details on specific token",
	RunE:  tokensGet,
}

func init() {
	CmdTokens.AddCommand(cmdTokensGet)
}

type data struct {
	Kind       string
	Namespaces []string
	Metadata   map[string]string
	Created    string
	Expires    string
}

func tokensGet(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving token details")
	cl.State.Fmt.Finish()

	var input string

	fmt.Print("Please paste the token to retrieve: ")
	fmt.Scanln(&input)

	cl.State.NewFormatter()

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetToken(ctx, &proto.GetTokenRequest{
		Token: input,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	token := models.Token{}
	token.FromProto(resp.Details)

	output, err := formatToken(&token, cl.State.Config.Detail)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not render token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(output)
	cl.State.Fmt.Finish()
	return nil
}

func formatToken(token *models.Token, detail bool) (string, error) {
	data := data{
		Kind:       color.BlueString(formatTokenKind(string(token.Kind))),
		Namespaces: token.Namespaces,
		Metadata:   token.Metadata,
		Created:    format.UnixMilli(token.Created, "Never", detail),
		Expires:    format.UnixMilli(token.Expires, "Never", detail),
	}

	const formatTmpl = `{{.Kind}} Token :: Created {{.Created}}
	{{- if .Namespaces}}

  Valid for Namespaces:
  {{- range $space := .Namespaces}}
    • {{ $space }}
  {{- end -}}
  {{- end -}}
  {{- if .Metadata}}

  Metadata:
    {{- range $key, $value := .Metadata}}
    • {{ $key }}: {{ $value }}
	{{- end -}}
  {{- end}}

  Expires: {{.Expires}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
