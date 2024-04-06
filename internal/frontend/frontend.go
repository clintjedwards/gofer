package frontend

import (
	"embed"
	"io/fs"
	"net/http"

	"github.com/rs/zerolog/log"
	"github.com/shurcooL/httpgzip"
)

// We bake frontend files directly into the binary
// embeddedAssets is an implementation of an http.filesystem
// that points to the public folder
//
//go:embed public
var embeddedAssets embed.FS

func StaticHandler() http.Handler {
	fsys, err := fs.Sub(embeddedAssets, "public")
	if err != nil {
		log.Fatal().Err(err).Msg("could not get embedded filesystem")
	}

	return httpgzip.FileServer(http.FS(fsys), httpgzip.FileServerOptions{IndexHTML: true})
}

func LocalHandler() http.Handler {
	return http.FileServer(http.Dir("./internal/frontend/public"))
}
