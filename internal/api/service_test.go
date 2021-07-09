package api

import (
	"testing"
)

func TestEncryptDecrypt(t *testing.T) {
	key := "mysupersecretkeymysupersecretkey"
	text := "mysupersecretvalue"

	encvalue, err := encrypt([]byte(key), []byte(text))
	if err != nil {
		t.Fatal(err)
	}

	decvalue, err := decrypt([]byte(key), encvalue)
	if err != nil {
		t.Fatal(err)
	}

	if text != string(decvalue) {
		t.Errorf("expected value mismatch; got %q want %q", decvalue, text)
	}
}
