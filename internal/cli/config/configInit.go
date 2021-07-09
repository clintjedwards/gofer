package config

import (
	"fmt"
	"os"

	_ "embed"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/spf13/cobra"
)

var cmdConfigInit = &cobra.Command{
	Use:   "init",
	Short: "Create example pipeline config file",
	Long: `Create example pipeline configuration file.

This file can be used as a example starting point and be customized further.

The default filename is example.gofer.hcl, but can be renamed via flags.`,
	Example: `$ gofer config init
$ gofer config -f myPipeline.hcl`,
	RunE: InitConfig,
}

//go:embed examplePipeline.hcl
var content string

func init() {
	cmdConfigInit.Flags().StringP("filepath", "f", "./example.gofer.hcl", "path to file")
	CmdConfig.AddCommand(cmdConfigInit)
}

func InitConfig(cmd *cobra.Command, _ []string) error {
	filepath, _ := cmd.Flags().GetString("filepath")

	cl.State.Fmt.Print("Creating pipeline file")

	err := createPipelineFile(filepath)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not create pipeline file: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Created pipeline file: %s", filepath))
	cl.State.Fmt.Finish()
	return nil
}

func createPipelineFile(name string) error {
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
