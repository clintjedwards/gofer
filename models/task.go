package models

type TaskKind string

// A task represents a single unit of execution in Gofer. Tasks can be either Custom Tasks or Common Tasks which
// each have their own advantages and disadvantages.
type Task interface {
	isTask()
	GetID() string
	GetDescription() string
	GetImage() string
	GetRegistryAuth() *RegistryAuth
	GetDependsOn() map[string]RequiredParentStatus
	GetVariables() []Variable
	GetEntrypoint() *[]string
	GetCommand() *[]string
	GetInjectAPIToken() bool
}
