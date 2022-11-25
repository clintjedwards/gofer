package api

import (
	"fmt"
)

const (
	ObjectPipelineKeyFmt    = "%s_%s_%s"         // <namespaceid>_<pipelineid>_<key>
	ObjectRunKeyFmt         = "%s_%s_%d_%s"      // <namespaceid>_<pipelineid>_<runid>_<key>
	GlobalSecretKeyFmt      = "global_secret_%s" // global_secret<key>
	PipelineSecretKeyFmt    = "%s_%s_%s"         // <namespaceid>_<pipelineid>_<key>
	TaskContainerIDFmt      = "%s_%s_%d_%s"      // <namespaceid>_<pipelineid>_<runid>_<taskrunid>
	ExtensionContainerIDFmt = "extension_%s"     // extension_<name>
	InstallerContainerIDFmt = "installer_%s"     // installer_<randomly-generated-value>
	TaskRunFilePath         = "%s/%s_%s_%d_%s"   // folder/<namespaceid>_<pipelineid>_<runid>_<taskrunid>
)

func globalSecretKey(key string) string {
	return fmt.Sprintf(GlobalSecretKeyFmt, key)
}

func pipelineSecretKey(namespace, pipeline, key string) string {
	return fmt.Sprintf(PipelineSecretKeyFmt, namespace, pipeline, key)
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

func extensionContainerID(name string) string {
	return fmt.Sprintf(ExtensionContainerIDFmt, name)
}

func installerContainerID() string {
	return fmt.Sprintf(InstallerContainerIDFmt, generateToken(5))
}

func taskRunLogFilePath(dir, namespace, pipeline string, run int64, task string) string {
	return fmt.Sprintf(TaskRunFilePath, dir, namespace, pipeline, run, task)
}
