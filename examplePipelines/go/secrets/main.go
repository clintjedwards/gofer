package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("secrets", "Secrets Pipeline").
		Description(
			"This pipeline shows how to pass pipeline and global secrets to a task. Gofer swaps in the real " +
				"values when the task starts, so they never show up in the pipeline config.").
		Tasks(
			sdk.NewTask("simple-task", "ghcr.io/clintjedwards/gofer/debug/log:latest").
				Description("This task has no dependencies so it will run immediately").
				Variables(map[string]string{
					"SOME_VARIABLE":             "something here",
					"ALTERNATE_LOGS_HEADER":     sdk.PipelineSecret("alternate_logs_header"),
					"SOME_GLOBAL_SECRET_I_NEED": sdk.GlobalSecret("test_secret_global"),
				})).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
