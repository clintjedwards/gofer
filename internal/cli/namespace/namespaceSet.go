package namespace

import (
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"

	"github.com/spf13/cobra"
)

var cmdNamespaceSet = &cobra.Command{
	Use:     "set <id>",
	Short:   "Set the target namespace",
	Example: `$ gofer namespace set my_namespace`,
	RunE:    namespaceSet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdNamespace.AddCommand(cmdNamespaceSet)
}

func namespaceSet(_ *cobra.Command, args []string) error {
	id := args[0]

	// In order to change the configuration we must update he in-memory
	// snapshot of the user's CLI configuration and then write it back
	// out to a file.
	cl.State.Config.Namespace = id
	err := cl.State.WriteConfig()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Namespace set to %q", id))
	cl.State.Fmt.Finish()
	return nil
}
