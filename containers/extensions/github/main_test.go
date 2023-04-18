package main

import (
	"context"
	"testing"

	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/google/go-cmp/cmp"
)

func TestMatchSubscriptions(t *testing.T) {
	extension := extension{
		subscriptions: map[string]map[string][]pipelineSubscription{},
	}

	_, err := extension.Subscribe(context.Background(), &proto.ExtensionSubscribeRequest{
		PipelineExtensionLabel: "test_extension",
		NamespaceId:            "test_namespace",
		PipelineId:             "test_pipeline",
		Config: map[string]string{
			"events":     "push,create",
			"repository": "clintjedwards/experimental",
		},
	})
	if err != nil {
		t.Fatal(err)
	}

	subs1 := extension.matchSubscriptions("create", "clintjedwards/experimental")
	subs2 := extension.matchSubscriptions("pull_request", "clintjedwards/experimental")

	result1 := []pipelineSubscription{
		{
			event:          "create",
			repository:     "clintjedwards/experimental",
			extensionLabel: "test_extension",
			namespace:      "test_namespace",
			pipeline:       "test_pipeline",
		},
	}

	result2 := []pipelineSubscription{}

	if diff := cmp.Diff(result1, subs1, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
		t.Errorf("mismatch (-want +got):\n%s", diff)
	}

	if diff := cmp.Diff(result2, subs2, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
		t.Errorf("mismatch (-want +got):\n%s", diff)
	}
}
