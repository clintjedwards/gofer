package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDTokens(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	token := Token{
		Hash:       "HASH_STR",
		Created:    0,
		Kind:       "KIND_STR",
		Namespaces: "NAMESPACE_STR",
		Metadata:   "METADATA_STR",
		Expires:    0,
		Disabled:   true,
	}

	id, err := db.InsertToken(db, &token)
	if err != nil {
		t.Fatal(err)
	}

	if id != 1 {
		t.Fatalf("id is incorrect integer")
	}

	token.ID = id

	tokens, err := db.ListTokens(db, 0, 0)
	if err != nil {
		t.Fatal(err)
	}

	if len(tokens) != 1 {
		t.Errorf("expected 1 element in list found %d", len(tokens))
	}

	if diff := cmp.Diff(token, tokens[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedToken, err := db.GetTokenByHash(db, token.Hash)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(token, fetchedToken); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	token.Disabled = false

	err = db.EnableToken(db, token.Hash)
	if err != nil {
		t.Fatal(err)
	}

	fetchedToken, err = db.GetTokenByHash(db, token.Hash)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(token, fetchedToken); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	token.Disabled = true

	err = db.DisableToken(db, token.Hash)
	if err != nil {
		t.Fatal(err)
	}

	fetchedToken, err = db.GetTokenByHash(db, token.Hash)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(token, fetchedToken); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteTokenByHash(db, token.Hash)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetTokenByHash(db, token.Hash)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}
