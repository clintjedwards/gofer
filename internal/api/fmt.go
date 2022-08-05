package api

import (
	"fmt"
)

const (
	ObjectPipelineKeyFmt    = "%s_%s_%s"       // <namespaceid>_<pipelineid>_<key>
	ObjectRunKeyFmt         = "%s_%s_%d_%s"    // <namespaceid>_<pipelineid>_<runid>_<key>
	SecretKeyFmt            = "%s_%s_%s"       // <namespaceid>_<pipelineid>_<key>
	TaskContainerIDFmt      = "%s_%s_%d_%s"    // <namespaceid>_<pipelineid>_<runid>_<taskrunid>
	TriggerContainerIDFmt   = "trigger_%s"     // trigger_<name>
	InstallerContainerIDFmt = "installer_%s"   // installer_<randomly-generated-value>
	TaskRunFilePath         = "%s/%s_%s_%d_%s" // folder/<namespaceid>_<pipelineid>_<runid>_<taskrunid>
)

func secretKey(namespace, pipeline, key string) string {
	return fmt.Sprintf(SecretKeyFmt, namespace, pipeline, key)
}

func pipelineObjectKey(namespace, pipeline, key string) string {
	return fmt.Sprintf(ObjectPipelineKeyFmt, namespace, pipeline, key)
}

func runObjectKey(namespace, pipeline string, run int64, key string) string {
	return fmt.Sprintf(ObjectRunKeyFmt, namespace, pipeline, run, key)
}

func taskContainerID(namespace, pipeline string, run int64, taskRun string) string {
	return fmt.Sprintf(TaskContainerIDFmt, namespace, pipeline, run, taskRun)
}

func triggerContainerID(name string) string {
	return fmt.Sprintf(TriggerContainerIDFmt, name)
}

func installerContainerID() string {
	return fmt.Sprintf(InstallerContainerIDFmt, generateToken(5))
}

func taskRunLogFilePath(dir, namespace, pipeline string, run int64, task string) string {
	return fmt.Sprintf(TaskRunFilePath, dir, namespace, pipeline, run, task)
}
