package models

type TaskKind string

const (
	TaskKindUnknown TaskKind = "UNKNOWN"

	// A Common Task is a pre-configured task used to give users quick access to common utilities they
	// might want in their pipeline.
	TaskKindCommon TaskKind = "COMMON"

	// A Custom Task is a task created solely by the pipeline user.
	TaskKindCustom TaskKind = "CUSTOM"
)

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
