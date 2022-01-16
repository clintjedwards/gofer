package bolt

import (
	"os"
	"testing"
)

func TestBolt(t *testing.T) {
	store, err := New("/tmp/test_bolt_secretStore.db", "testencryptionkeytestencryptionk")
	if err != nil {
		t.Fatal(err)
	}

	err = store.PutSecret("testkey", "mysupersecretkey", false)
	if err != nil {
		t.Fatal(err)
	}

	secret, err := store.GetSecret("testkey")
	if err != nil {
		t.Fatal(err)
	}

	if secret != "mysupersecretkey" {
		t.Fatal("secret returns does not equal secret put in")
	}

	defer os.Remove("/tmp/test_bolt_secretStore.db")
}
