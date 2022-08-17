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
	defer os.Remove("/tmp/test_bolt_secretStore.db")

	err = store.PutSecret("testkey1", "mysupersecretkey", false)
	if err != nil {
		t.Fatal(err)
	}

	err = store.PutSecret("testkey2", "myothersupersecretkey", false)
	if err != nil {
		t.Fatal(err)
	}

	err = store.PutSecret("differentkey2", "mynextsupersecretkey", false)
	if err != nil {
		t.Fatal(err)
	}

	secret, err := store.GetSecret("testkey1")
	if err != nil {
		t.Fatal(err)
	}

	if secret != "mysupersecretkey" {
		t.Fatal("secret returns does not equal secret put in")
	}

	keys, err := store.ListSecretKeys("testkey")
	if err != nil {
		t.Fatal(err)
	}

	if len(keys) != 2 {
		t.Fatalf("expected two keys got %d", len(keys))
	}
}
