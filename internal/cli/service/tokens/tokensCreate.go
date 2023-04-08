package tokens

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTokensCreate = &cobra.Command{
	Use:   "create <management|client>",
	Short: "Create new API token",
	RunE:  tokensCreate,
	Args:  cobra.ExactArgs(1),
}

func init() {
	cmdTokensCreate.Flags().StringP("expiry", "e", "24h", "set the expiration time of the key. This value uses golang time durations such as '300ms', '1.5h' or '2h45m'. https://pkg.go.dev/time#ParseDuration")
	cmdTokensCreate.Flags().StringSliceP("namespaces", "n", []string{"default"}, "namespaces this key will have access to. If not specified namespace is 'default'. Namespace string can be regex for multiple and continual matching.")
	cmdTokensCreate.Flags().StringSliceP("metadata", "m", []string{}, "metadata about the token, useful for attaching a name, team, and other details. Format = key:value")
	CmdTokens.AddCommand(cmdTokensCreate)
}

func metadataToMap(metadata []string) map[string]string {
	metadataMap := map[string]string{}

	for _, keyPairString := range metadata {
		key, value, ok := strings.Cut(keyPairString, ":")
		if !ok {
			continue
		}

		metadataMap[key] = value
	}

	return metadataMap
}

func tokensCreate(cmd *cobra.Command, args []string) error {
	expiry, _ := cmd.Flags().GetString("expiry")
	namespaces, _ := cmd.Flags().GetStringSlice("namespaces")
	metadataSlice, _ := cmd.Flags().GetStringSlice("metadata")
	tokenMetadata := metadataToMap(metadataSlice)

	cl.State.Fmt.Print("Creating Token")

	kind := args[0]
	if kind != "management" && kind != "client" {
		cl.State.Fmt.PrintErr(fmt.Sprintf("invalid kind %q", kind))
		cl.State.Fmt.Finish()
		return fmt.Errorf("invalid kind")
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
	resp, err := client.CreateToken(ctx, &proto.CreateTokenRequest{
		Kind:       proto.CreateTokenRequest_Kind(proto.CreateTokenRequest_Kind_value[string(kind)]),
		Metadata:   tokenMetadata,
		Namespaces: namespaces,
		Expires:    expiry,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Token: %s", resp.Token))
	cl.State.Fmt.Finish()

	return nil
}
