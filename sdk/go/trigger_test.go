package sdk

import (
	"encoding/json"
	"testing"

	"github.com/google/go-cmp/cmp"
)

// IMPORTANT: Changing the `expected` string in this function requires you to change the rust sister function in
// the rust sdk.
func TestInstallInstructions(t *testing.T) {
	instructions, err := NewInstructionsBuilder().AddMessage("test").AddQuery("test", "config").JSON()
	if err != nil {
		t.Fatal(err)
	}

	expected := `{"instructions":[{"message":{"text":"test"}},{"query":{"text":"test","config_key":"config"}}]}`

	if diff := cmp.Diff(expected, instructions); diff != "" {
		t.Errorf("unexpected json output (-want +got):\n%s", diff)
	}
}

func TestInstallInstructionsUnmarshal(t *testing.T) {
	input := `{"instructions":[{"message":{"text":"test"}},{"query":{"text":"test","config_key":"config"}}]}`

	expected := InstallInstructions{
		Instructions: []isInstallInstruction{
			InstallInstructionMessageWrapper{
				Message: InstallInstructionMessage{
					Text: "test",
				},
			},
			InstallInstructionQueryWrapper{
				Query: InstallInstructionQuery{
					Text:      "test",
					ConfigKey: "config",
				},
			},
		},
	}

	testStruct := InstallInstructions{}
	err := json.Unmarshal([]byte(input), &testStruct)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(expected, testStruct); diff != "" {
		t.Errorf("unexpected json output (-want +got):\n%s", diff)
	}
}
