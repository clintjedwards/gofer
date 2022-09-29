package triggers

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"
	sdk "github.com/clintjedwards/gofer/sdk/go"
	"github.com/fatih/color"
	"golang.org/x/text/cases"
	"golang.org/x/text/language"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTriggersInstall = &cobra.Command{
	Use:   "install <name> <image>",
	Short: "Install a specific trigger by name.",
	Long: `Install a specific trigger by name.

Gofer allows you to install the triggers either manually or by following prompts provided by the trigger.
By not using the "--manual" flag Gofer will attempt to collect trigger installation information and then prompt
the user.

By simply following the prompt in this method the Gofer CLI will collect the necessary parameters require to setup
the trigger. It will then attempt to install the trigger on your behalf.

When using the --manual flag you'll need to provide config values via the "-c" flag in KEY=VALUE format.`,
	Example: `$ gofer triggers install cron ghcr.io/clintjedwards/gofer/triggers/cron:latest
$ gofer triggers install interval ghcr.io/clintjedwards/gofer/triggers/interval:latest --manual -c MIN_DURATION=1m`,
	RunE: triggersInstall,
	Args: cobra.ExactArgs(2),
}

func init() {
	cmdTriggersInstall.Flags().BoolP("manual", "m", false, "manually set up the trigger by providing settings via the '-s' flag")
	cmdTriggersInstall.Flags().StringSliceP("config", "c", []string{}, "provide trigger config values for installation")
	CmdTriggers.AddCommand(cmdTriggersInstall)
}

func triggersInstall(cmd *cobra.Command, args []string) error {
	name := args[0]
	image := args[1]

	cl.State.Fmt.Print("Installing trigger")

	manual, err := cmd.Flags().GetBool("manual")
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

	if manual && len(configList) > 0 {
		cl.State.Fmt.PrintErr("cannot use both the manual flag and config flag at the same time")
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

	if !manual {
		cl.State.Fmt.Print("Retrieving trigger install instructions")

		ctx := metadata.NewOutgoingContext(context.Background(), md)
		resp, err := client.GetTriggerInstallInstructions(ctx, &proto.GetTriggerInstallInstructionsRequest{
			Image: image,
			// TODO(clintjedwards): This needs registry auth
		})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not get trigger install instructions: %v", err))
			cl.State.Fmt.Finish()
			return err
		}

		cl.State.Fmt.PrintSuccess("Downloaded trigger install instructions")
		cl.State.Fmt.Println("Parsing install instructions")

		instructionsString := strings.TrimSpace(resp.Instructions)
		instructions := sdk.InstallInstructions{}

		err = json.Unmarshal([]byte(instructionsString), &instructions)
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not parse trigger install instructions: %v", err))
			cl.State.Fmt.Finish()
			return err
		}

		cl.State.Fmt.PrintSuccess("Parsed trigger install instructions")
		cl.State.Fmt.Finish()

		// Enter alternate screen
		fmt.Print("\x1b[?1049h")

		toTitle := cases.Title(language.AmericanEnglish)
		fmt.Printf(":: %s Trigger Setup\n", color.CyanString(toTitle.String(name)))

		for _, instruction := range instructions.Instructions {
			switch v := instruction.(type) {
			case sdk.InstallInstructionMessageWrapper:
				fmt.Println(strings.TrimSpace(v.Message.Text))
			case sdk.InstallInstructionQueryWrapper:
				var input string
				fmt.Printf("> %s:", strings.TrimSpace(v.Query.Text))
				fmt.Scanln(&input)
				configMap[v.Query.ConfigKey] = strings.TrimSpace(input)
			}
		}

		fmt.Printf("Install trigger %q with above settings? [Y/n]: ", toTitle.String(name))
		var input string
		fmt.Scanln(&input)

		if !strings.EqualFold(input, "y") {
			fmt.Print("\x1b[?1049l")
			cl.State.NewFormatter()
			cl.State.Fmt.PrintErr("User aborted installation process")
			cl.State.Fmt.Finish()
			return fmt.Errorf("user aborted installation process")
		}

		fmt.Print("\x1b[?1049l")
		cl.State.NewFormatter()
	} else {
		for _, config := range configList {
			key, value, ok := strings.Cut(config, "=")
			if !ok {
				cl.State.Fmt.PrintErr("Key-value pair malformed; should be in format: <key>=<value>")
				cl.State.Fmt.Finish()
				return fmt.Errorf("malformed input")
			}

			configMap[key] = value
		}
	}

	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.InstallTrigger(ctx, &proto.InstallTriggerRequest{
		Name:      name,
		Image:     image,
		Variables: configMap,
		// TODO(clintjedwards): Support registry auth
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not install trigger: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Trigger Installed!")
	cl.State.Fmt.Finish()

	return nil
}
