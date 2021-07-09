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

	// Secrets path is the file which container secrets that will be ingested by the docker container.
	// Since local docker(without swarm) doesn't have a coherent way to store secrets we create one by
	// giving the user the option to create those secrets on a per-line basis in a file.
	SecretsPath string `split_words:"true" hcl:"secrets_path,optional"`
}

func DefaultDockerConfig() *Docker {
	return &Docker{
		Prune:         false,
		PruneInterval: mustParseDuration("24h"),
	}
}
