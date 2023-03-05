package service

import (
	"fmt"
	"os"

	_ "embed"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/spf13/cobra"
)

var cmdServiceInitConfig = &cobra.Command{
	Use:   "init-config",
	Short: "Create example Gofer server config file.",
	Long: `Create example Gofer server config file.

This file can be used as a example starting point and be customized further. This file should not
be used to run production versions of Gofer as it is inherently insecure.

The default filename is example.gofer.hcl, but can be renamed via flags.`,
	Example: `$ gofer service init-config
$ gofer service init-config -f myServer.hcl`,
	RunE: serviceInitConfig,
}

//go:embed sampleConfig.hcl
var content string

func init() {
	cmdServiceInitConfig.Flags().StringP("filepath", "f", "./example.gofer.hcl", "path to file")
	CmdService.AddCommand(cmdServiceInitConfig)
}

func serviceInitConfig(cmd *cobra.Command, _ []string) error {
	filepath, _ := cmd.Flags().GetString("filepath")

	cl.State.Fmt.Print("Creating service config file")

	err := createServiceConfigFile(filepath)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not create service config file: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Created service config file: %s", filepath))
	cl.State.Fmt.Finish()
	return nil
}

func createServiceConfigFile(name string) error {
	file, err := os.Create(name)
	if err != nil {
		return err
	}
	defer file.Close()

	_, err = file.WriteString(content)
	if err != nil {
		return err
	}

	return nil
}
