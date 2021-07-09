package api

import (
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestMergeMaps(t *testing.T) {
	first := map[string]string{"test1": "value1", "test2": "value2", "test3": "value3"}
	second := map[string]string{"test1": "value"}
	third := map[string]string{"test2": "valuethird"}

	expected := map[string]string{"test1": "value", "test2": "valuethird", "test3": "value3"}

	if diff := cmp.Diff(expected, mergeMaps(first, second, third)); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}
}
