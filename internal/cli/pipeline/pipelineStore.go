package pipeline

import (
	"github.com/spf13/cobra"
)

var CmdPipelineStore = &cobra.Command{
	Use:   "store",
	Short: "Store pipeline specific values",
	Long: `Store pipeline specific values.

Gofer has two ways to temporarily store objects that might be useful.

This command allows users to store objects at the "pipeline" level in a key-object fashion. Pipeline level objects are
great for storing things that need to be cached over many runs and don't change very often.

Pipeline objects are kept forever until the limit of number of pipeline objects is reached(this may be different depending on configuration).
Once this limit is reached the _oldest_ object will be removed to make space for the new object.

This "oldest is evicted" rule does not apply to objects which are being overwritten. So replacing an already populated key with
a newer object would not cause any object deletions even at the object limit.`,
}

func init() {
	CmdPipeline.AddCommand(CmdPipelineStore)
}
