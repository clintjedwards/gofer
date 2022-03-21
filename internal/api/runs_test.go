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

func TestParseInterpolationSyntax(t *testing.T) {
	tests := map[string]struct {
		kind     string
		value    string
		expected string
	}{
		"secret":         {kind: "secret", value: "secret{{example}}", expected: "example"},
		"pipeline":       {kind: "pipeline", value: "pipeline{{example}}", expected: "example"},
		"run":            {kind: "run", value: "run{{example}}", expected: "example"},
		"incorrect_kind": {kind: "secret", value: "run{{example}}", expected: "run{{example}}"},
		"normal_value":   {kind: "secret", value: "normal_value", expected: "normal_value"},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			result := parseInterpolationSyntax(test.kind, test.value)

			if result != test.expected {
				t.Errorf("incorrect interpolation result; want %s got %s", test.expected, result)
			}
		})
	}
}
