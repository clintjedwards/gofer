package notifier

import (
	"github.com/spf13/cobra"
)

var CmdNotifier = &cobra.Command{
	Use:   "notifier",
	Short: "Get details about Gofer notifiers",
	Long: `Get details about Gofer notifiers.

Notifiers act as plugins for Gofer that execute a container as the last step in your pipeline process.

For example, reporting the status of the pipeline to Slack would be a good case to use a notifier.`,
}
