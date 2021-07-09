package api

import (
	"crypto/tls"
	"fmt"
	"io/ioutil"

	_ "embed"
)

// We use these functions to supply TLS for various services that require it. To make development easy
// we bake in general localhost certs for quick bootstrap. The server will not start with dev certs loaded
// unless explicitly told to do so with devmode=true.

//go:embed localhost.crt
var devtlscert []byte

//go:embed localhost.key
var devtlskey []byte

// generateTLSConfig returns TLS config object necessary for HTTPS loaded from files. If server is in devmode and
// no cert is provided it instead loads certificates from embedded files for ease of development.
func (api *API) generateTLSConfig(certPath, keyPath string) (*tls.Config, error) {
	var serverCert tls.Certificate
	var err error

	if api.config.Server.DevMode && certPath == "" {
		serverCert, err = tls.X509KeyPair(devtlscert, devtlskey)
		if err != nil {
			return nil, err
		}
	} else {
		if certPath == "" || keyPath == "" {
			return nil, fmt.Errorf("TLS cert and key cannot be empty")
		}

		serverCert, err = tls.LoadX509KeyPair(certPath, keyPath)
		if err != nil {
			return nil, err
		}
	}

	tlsConfig := &tls.Config{
		Certificates: []tls.Certificate{serverCert},
		ClientAuth:   tls.NoClientCert,
	}

	return tlsConfig, nil
}

// getTLSFiles returns certificates suppled from file paths. If server is in devmode and no cert is provided
// it instead loads certificates from embedded files for ease of development.
func (api *API) getTLSFromFile(certPath, keyPath string) (cert, key []byte, err error) {
	if api.config.Server.DevMode && certPath == "" {
		return devtlscert, devtlskey, nil
	}

	if certPath == "" || keyPath == "" {
		return nil, nil, fmt.Errorf("TLS cert and key cannot be empty")
	}

	cert, err = ioutil.ReadFile(certPath)
	if err != nil {
		return nil, nil, err
	}

	key, err = ioutil.ReadFile(keyPath)
	if err != nil {
		return nil, nil, err
	}

	return cert, key, nil
}
