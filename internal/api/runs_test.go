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

func TestParseInterpolationSyntaxKnownGood(t *testing.T) {
	tests := map[string]struct {
		kind     InterpolationKind
		value    string
		expected string
	}{
		"pipeline secret": {kind: InterpolationKindPipelineSecret, value: "pipeline_secret{{example}}", expected: "example"},
		"global secret":   {kind: InterpolationKindGlobalSecret, value: "global_secret{{example}}", expected: "example"},
		"pipeline object": {kind: InterpolationKindPipelineObject, value: "pipeline_object{{example}}", expected: "example"},
		"run object":      {kind: InterpolationKindRunObject, value: "run_object{{example}}", expected: "example"},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			result, err := parseInterpolationSyntax(test.kind, test.value)
			if err != nil {
				t.Fatal(err)
			}

			if result != test.expected {
				t.Errorf("incorrect interpolation result; want %s got %s", test.expected, result)
			}
		})
	}
}

func TestParseInterpolationSyntaxKnownBad(t *testing.T) {
	tests := map[string]struct {
		kind     InterpolationKind
		value    string
		expected string
	}{
		"incorrect_kind": {kind: InterpolationKindPipelineSecret, value: "run{{example}}", expected: ""},
		"normal_value":   {kind: InterpolationKindGlobalSecret, value: "normal_value", expected: ""},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			result, err := parseInterpolationSyntax(test.kind, test.value)
			if err == nil {
				t.Fatalf("test %q should have returned an err but instead returned a successful value (%q)", name, result)
			}

			if result != test.expected {
				t.Fatalf("incorrect interpolation result; want %s got %s", test.expected, result)
			}
		})
	}
}
