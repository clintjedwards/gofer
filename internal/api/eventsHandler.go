package api

import (
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"

	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
	"github.com/gorilla/mux"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

func (api *API) eventsHandler(w http.ResponseWriter, req *http.Request) {
	vars := mux.Vars(req)
	triggerKind := vars["trigger"]
	trigger, exists := api.triggers[triggerKind]
	if !exists {
		sendErrResponse(w, http.StatusBadRequest, fmt.Errorf("trigger %q does not exist", triggerKind))
		return
	}

	request, err := ioutil.ReadAll(req.Body)
	if err != nil {
		sendErrResponse(w, http.StatusBadRequest, err)
		return
	}
	defer req.Body.Close()

	conn, err := grpcDial(trigger.URL)
	if err != nil {
		log.Error().Err(err).Str("trigger", triggerKind).Msg("could not connect to trigger")
	}
	defer conn.Close()

	client := sdkProto.NewTriggerClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(trigger.Key))
	_, err = client.ExternalEvent(ctx, &sdkProto.ExternalEventRequest{
		Payload: request,
	})
	if err != nil {
		if status.Code(err) == codes.Canceled {
			return
		}

		log.Error().Err(err).Str("trigger", triggerKind).Msg("could not connect to trigger")
		sendErrResponse(w, http.StatusInternalServerError, fmt.Errorf("could not connect to trigger"))
		return
	}
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
