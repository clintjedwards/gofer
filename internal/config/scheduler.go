package config

import "time"

// Scheduler defines config settings for gofer scheduler. The scheduler is the backend for how containers are run.
type Scheduler struct {
	// The database engine used by the scheduler
	// possible values are: docker
	Engine string  `hcl:"engine,optional"`
	Docker *Docker `hcl:"docker,block"`
}

func DefaultSchedulerConfig() *Scheduler {
	return &Scheduler{
		Engine: "docker",
		Docker: DefaultDockerConfig(),
	}
}

type Docker struct {
	// Prune runs a reoccuring `docker system prune` job to avoid filling the local disk with docker images.
	Prune bool `hcl:"prune,optional"`

	// The period of time in between runs of `docker system prune`
	PruneInterval time.Duration `split_words:"true"`

	// PruneIntervalHCL is the HCL compatible counter part to PruneInterval. It allows the parsing of a string
	// to a time.Duration since HCL does not support parsing directly into a time.Duration.
	PruneIntervalHCL string `ignored:"true" hcl:"prune_interval,optional"`
}

func DefaultDockerConfig() *Docker {
	return &Docker{
		Prune:         false,
		PruneInterval: mustParseDuration("24h"),
	}
}
