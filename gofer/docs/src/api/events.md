# Events

Gofer emits events for actions that happen within it's purview. You can use the event api to get a list of all events or request specific events.

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-ws">WS</span> | [`/api/events`](#list-all-events) |
| <span class="api-method api-method-get">GET</span> | [`/api/events/{event_id}`](#get-api-event-by-id) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/events/{event_id}`](#delete-api-event-by-id) |

---

## List all events

<div class="api-endpoint"><span class="api-method api-method-ws">WS</span><code>/api/events</code></div>

This endpoint upgrades the connection to a websocket: after the `101` response the server streams data over the socket instead of returning a body.

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `history` | boolean | no | If set to true Gofer first exhausts events that have already passed before it starts to stream new events. |
| `reverse` | boolean | no | Reverses the order of events by the time they were emitted. By default Gofer lists events in ascending order; setting reverse to true causes events to be in descending order. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `101` | none | Negotiating protocol upgrade from HTTP/1.1 to WebSocket |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get api event by id

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/events/{event_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `event_id` | string | yes | The unique identifier for the target event. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetEventResponse](./schemas.md#geteventresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete api event by id

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/events/{event_id}</code></div>

This route is only accessible by admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `event_id` | string | yes | The unique identifier for the target event. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

