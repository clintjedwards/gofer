package extension

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdExtensionInstall = &cobra.Command{
	Use:   "install <name> <image>",
	Short: "Install a Gofer extension",
	Long: `Install a specific Gofer extension.

Gofer allows you to install the extensions either manually or by following a series of prompts provided by the extension.

By using the "--interactive" flag Gofer will provide installation instructions and prompts provided by the extension.
It will then attempt to install the extension on your behalf.

When using the --manual flag you'll need to provide config values via the "-c" flag in KEY=VALUE format.
Normally the extension documentation will have the correct config values needed.`,
	Example: `$ gofer extension install cron ghcr.io/clintjedwards/gofer/extensions/cron:latest -c MIN_DURATION=1m
$ gofer extension install interval ghcr.io/clintjedwards/gofer/extensions/interval:latest --interactive`,
	RunE: extensionInstall,
	Args: cobra.ExactArgs(2),
}

func init() {
	cmdExtensionInstall.Flags().BoolP("interactive", "i", false, "Attempt to set up the extension by querying the extension for config information.")
	cmdExtensionInstall.Flags().StringSliceP("config", "c", []string{}, "provide extension config values for installation")
	CmdExtension.AddCommand(cmdExtensionInstall)
}

func extensionInstall(cmd *cobra.Command, args []string) error {
	name := args[0]
	image := args[1]

	cl.State.Fmt.Print("Installing extension")

	interactive, err := cmd.Flags().GetBool("interactive")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	configList, err := cmd.Flags().GetStringSlice("config")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	if interactive && len(configList) > 0 {
		cl.State.Fmt.PrintErr("Cannot use both the interactive flag and config flag at the same time")
		cl.State.Fmt.Finish()
		return fmt.Errorf("flag mismatch")
	}

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)

	configMap := map[string]string{}

	if interactive {
		cl.State.Fmt.Print("Attaching to extension to retrieve installation information for interactive mode")

		ctx := metadata.NewOutgoingContext(context.Background(), md)
		interactiveConn, err := client.RunExtensionInstaller(ctx)
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("Could not connect to extension to get installation instructions %v", err))
			cl.State.Fmt.Finish()
			return err
		}

		interactiveConn.Send(&proto.RunExtensionInstallerClientMessage{
			MessageType: &proto.RunExtensionInstallerClientMessage_Init_{
				Init: &proto.RunExtensionInstallerClientMessage_Init{
					Image: image,
					// TODO(clintjedwards): This needs registry auth
				},
			},
		})

		cl.State.Fmt.PrintSuccess("Attached to extension container " + image)

		for {
			msg, err := interactiveConn.Recv()
			if err != nil {
				// If we got an EOF error then the extension closed the connection.
				if strings.Contains(err.Error(), "EOF") {
					break
				}

				// If the context was cancelled, that means that the extension is done and we should process the installation.
				if strings.Contains(err.Error(), "context canceled") {
					break
				}

				// If the client disconnected, exit cleanly.
				if strings.Contains(err.Error(), "client disconnected") {
					break
				}

				cl.State.Fmt.PrintErr(fmt.Sprintf("Could not read from extension connection %v", err))
				cl.State.Fmt.Finish()
				return err
			}

			switch extensionMsg := msg.MessageType.(type) {
			case *proto.RunExtensionInstallerExtensionMessage_ConfigSetting_:
				configMap[extensionMsg.ConfigSetting.Config] = extensionMsg.ConfigSetting.Value
			case *proto.RunExtensionInstallerExtensionMessage_Msg:
				cl.State.Fmt.Println(extensionMsg.Msg)
			case *proto.RunExtensionInstallerExtensionMessage_Query:
				answer := cl.State.Fmt.PrintQuestion(extensionMsg.Query)
				err := interactiveConn.Send(&proto.RunExtensionInstallerClientMessage{
					MessageType: &proto.RunExtensionInstallerClientMessage_Msg{
						Msg: answer,
					},
				})
				if err != nil {
					cl.State.Fmt.PrintErr(fmt.Sprintf("Could not send answer to extension query back to extension %v", err))
					cl.State.Fmt.Finish()
					return err
				}
			default:
				cl.State.Fmt.PrintErr(fmt.Sprintf("Could not properly decode incoming message from extension; incorrect type %T", extensionMsg))
				cl.State.Fmt.Finish()
				return err
			}

		}

		cl.State.Fmt.PrintSuccess("Completed interactive extension routine")
	}

	if !interactive {
		for _, config := range configList {
			key, value, ok := strings.Cut(config, "=")
			if !ok {
				cl.State.Fmt.PrintErr("Key-value pair malformed; should be in format: <key>=<value>")
				cl.State.Fmt.Finish()
				return err
			}

			configMap[key] = value
		}
	}

	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.InstallExtension(ctx, &proto.InstallExtensionRequest{
		Name:      name,
		Image:     image,
		Variables: configMap,
		// TODO(clintjedwards): Support registry auth
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not install extension: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Extension Installed!")
	cl.State.Fmt.Finish()

	return nil
}
