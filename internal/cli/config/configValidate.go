package config

import (
	"fmt"
	"os"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/hashicorp/go-multierror"
	"github.com/spf13/cobra"
)

var cmdConfigValidate = &cobra.Command{
	Use:   "validate <...path>",
	Short: "Validate pipeline configuration files",
	Example: `$ gofer config validate myPipeline.hcl
$ gofer config validate myPipeline.hcl anotherFile.hcl
$ gofer config validate somedir/*`,
	RunE: configValidate,
	Args: cobra.MinimumNArgs(1),
}

func init() {
	CmdConfig.AddCommand(cmdConfigValidate)
}

func configValidate(_ *cobra.Command, args []string) error {
	cl.State.Fmt.Print("Validating pipeline configuration")

	for _, path := range args {
		cl.State.Fmt.Print(fmt.Sprintf("Processing file %q", path))

		content, err := os.ReadFile(path)
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not open pipeline file: %v", err))
			continue
		}

		hclConfig := models.HCLPipelineConfig{}
		err = hclConfig.FromBytes(content, path)
		if err != nil {
			cl.State.Fmt.PrintErr(err)
			continue
		}

		err = hclConfig.Validate()
		if err != nil {
			if merr, ok := err.(*multierror.Error); ok {
				cl.State.Fmt.PrintErr(fmt.Sprintf("Config %q has errors:", path))
				for _, err := range merr.Errors {
					cl.State.Fmt.PrintErr("  " + err.Error())
				}
			}
			continue
		}

		cl.State.Fmt.PrintSuccess(fmt.Sprintf("Config %q is valid!", path))
	}

	cl.State.Fmt.Finish()
	return nil
}
