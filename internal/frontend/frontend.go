package frontend

import (
	"embed"
	"io/fs"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/rs/zerolog/log"
	"github.com/shurcooL/httpgzip"
)

// We bake frontend files directly into the binary
// embeddedAssets is an implementation of an http.filesystem
// that points to the public folder
//
//go:embed public
var embeddedAssets embed.FS

// Frontend represents an instance of the frontend application
type Frontend struct{}

// New initializes a new UI application
func New() *Frontend {
	return &Frontend{}
}

// RegisterUIRoutes registers the endpoints needed for the frontend
// with an already established router
func (ui *Frontend) RegisterUIRoutes(router *mux.Router) {
	fsys, err := fs.Sub(embeddedAssets, "public")
	if err != nil {
		log.Fatal().Err(err).Msg("could not get embedded filesystem")
	}

	handler := httpgzip.FileServer(http.FS(fsys), httpgzip.FileServerOptions{IndexHTML: true})

	router.PathPrefix("/").Handler(handler)
}
