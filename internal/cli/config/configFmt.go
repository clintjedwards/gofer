package config

import (
	"fmt"
	"os"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/hashicorp/hcl/v2/hclwrite"
	"github.com/spf13/cobra"
)

var cmdConfigFmt = &cobra.Command{
	Use:   "fmt <...path>",
	Short: "Format pipeline configuration files",
	Long: `Format pipeline configuration files.

A basic HCL formatter. Rewrites the file in place.`,
	Example: `$ gofer config fmt myPipeline.hcl
$ gofer config fmt myPipeline.hcl anotherFile.hcl
$ gofer config fmt somedir/*`,
	RunE: configFmt,
	Args: cobra.MinimumNArgs(1),
}

func init() {
	CmdConfig.AddCommand(cmdConfigFmt)
}

func configFmt(_ *cobra.Command, args []string) error {
	cl.State.Fmt.Print("Formatting pipeline config")

	for _, path := range args {
		cl.State.Fmt.Print(fmt.Sprintf("Processing file %q", path))
		content, err := os.ReadFile(path)
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not open pipeline config file: %v", err))
			continue
		}

		result := hclwrite.Format(content)
		err = os.WriteFile(path, result, 0644)
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not write pipeline config file: %v", err))
			continue
		}

		cl.State.Fmt.PrintSuccess(fmt.Sprintf("Formatted file %q", path))
	}

	cl.State.Fmt.Finish()
	return nil
}
