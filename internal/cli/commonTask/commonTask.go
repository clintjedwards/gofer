package commonTask

import (
	"github.com/spf13/cobra"
)

var CmdCommonTask = &cobra.Command{
	Use:   "common-task",
	Short: "Get details about Gofer common tasks",
	Long: `Get details about Gofer common tasks.

Common Tasks act as plugins for Gofer that execute as normal tasks. They are useful because they can be set up
in advanced such that users don't have to deal with common settings like authentication.`,
}
