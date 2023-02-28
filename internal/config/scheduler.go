package config

import "time"

// Scheduler defines config settings for gofer scheduler. The scheduler is the backend for how containers are run.
type Scheduler struct {
	// The database engine used by the scheduler
	// possible values are: docker
	Engine string  `koanf:"engine"`
	Docker *Docker `koanf:"docker"`
}

func DefaultSchedulerConfig() *Scheduler {
	return &Scheduler{
		Engine: "docker",
		Docker: DefaultDockerConfig(),
	}
}

type Docker struct {
	// Prune runs a reoccuring `docker system prune` job to avoid filling the local disk with docker images.
	Prune bool `koanf:"prune"`

	// The period of time in between runs of `docker system prune`
	PruneInterval time.Duration `koanf:"prune_interval"`
}

func DefaultDockerConfig() *Docker {
	return &Docker{
		Prune:         false,
		PruneInterval: mustParseDuration("24h"),
	}
}
