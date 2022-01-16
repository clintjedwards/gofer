package api

import (
	"fmt"
)

const (
	SecretKeyFmt = "%s_%s_%s" // namespaceid_pipelineid_key
)

func secretKey(namespace, pipeline, key string) string {
	return fmt.Sprintf(SecretKeyFmt, namespace, pipeline, key)
}
