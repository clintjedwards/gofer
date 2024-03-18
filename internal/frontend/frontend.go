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
type Frontend struct {
	loadFromLocal bool
}

// New initializes a new UI application
func New(loadFromLocal bool) *Frontend {
	return &Frontend{
		loadFromLocal: loadFromLocal,
	}
}

// RegisterUIRoutes registers the endpoints needed for the frontend
// with an already established router
func (ui *Frontend) RegisterUIRoutes(router *mux.Router) {
	var handler http.Handler

	if ui.loadFromLocal {
		log.Warn().Msg("Loading frontend files from local disk path 'public'")
		handler = localHandler()
	} else {
		handler = staticHandler()
	}

	router.PathPrefix("/").Handler(handler)
}

func staticHandler() http.Handler {
	fsys, err := fs.Sub(embeddedAssets, "public")
	if err != nil {
		log.Fatal().Err(err).Msg("could not get embedded filesystem")
	}

	return httpgzip.FileServer(http.FS(fsys), httpgzip.FileServerOptions{IndexHTML: true})
}

func localHandler() http.Handler {
	return http.FileServer(http.Dir("./internal/frontend/public"))
}
