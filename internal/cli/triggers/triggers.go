package triggers

import (
	"github.com/spf13/cobra"
)

var CmdTriggers = &cobra.Command{
	Use:   "triggers",
	Short: "Get details about Gofer triggers",
	Long: `Get details about Gofer triggers.

Triggers act as plugins for Gofer that execute a run for a pipeline based on some criteria.

An example of a trigger might be the simply the passing of time for the "interval" trigger. A user will _subscribe_ to
this trigger in their pipeline configuration file and based on settings used in that file interval will alert Gofer
when the user's intended interval of time has passed. This automatically then kicks off a new instance of a run for
that specific pipeline.`,
}
