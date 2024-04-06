package api

import (
	"encoding/json"
	"net/http"

	"github.com/rs/zerolog/log"
)

func (api *APIContext) externalEventsHandler(w http.ResponseWriter, req *http.Request) {
	// vars := mux.Vars(req)
	// extensionKind := vars["extension"]
	// extension, exists := api.extensions.Get(extensionKind)
	// if !exists {
	// 	sendErrResponse(w, http.StatusBadRequest, fmt.Errorf("extension %q does not exist", extensionKind))
	// 	return
	// }

	// serializedRequest := &bytes.Buffer{}
	// err := req.Write(serializedRequest)
	// if err != nil {
	// 	sendErrResponse(w, http.StatusBadRequest, fmt.Errorf("could not serialize http request"))
	// 	return
	// }

	// defer req.Body.Close()

	// conn, err := grpcDial(extension.URL)
	// if err != nil {
	// 	log.Error().Err(err).Str("extension", extensionKind).Msg("could not connect to extension")
	// }
	// defer conn.Close()

	// client := proto.NewExtensionServiceClient(conn)

	// ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*extension.Key))
	// _, err = client.ExternalEvent(ctx, &proto.ExtensionExternalEventRequest{
	// 	Payload: serializedRequest.Bytes(),
	// })
	// if err != nil {
	// 	if status.Code(err) == codes.Canceled {
	// 		return
	// 	}

	// 	log.Error().Err(err).Str("extension", extensionKind).Msg("could not connect to extension")
	// 	sendErrResponse(w, http.StatusInternalServerError, fmt.Errorf("could not connect to extension"))
	// 	return
	// }
}

// sendErrResponse converts raw objects and parameters to a json response specifically for erorrs
// and passes it to a provided writer. The creation of a separate function for just errors,
// is due to how they are handled differently from other payload types.
func sendErrResponse(w http.ResponseWriter, httpStatusCode int, appErr error) {
	w.WriteHeader(httpStatusCode)

	enc := json.NewEncoder(w)
	err := enc.Encode(map[string]string{"err": appErr.Error()})
	if err != nil {
		log.Error().Err(err).Msgf("could not encode json response: %v", err)
	}
}
