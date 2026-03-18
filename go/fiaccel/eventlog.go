package fiaccel

import "encoding/json"

// EventProjectionConfig controls how raw event records are normalized.
type EventProjectionConfig struct {
	TimestampField   string
	EventTypeField   string
	ActorIDField     string
	DisableActorID   bool
	ActorTypeField   string
	DisableActorType bool
	DataField        string
	Limit            int
}

type eventProjectionConfigPayload struct {
	TimestampField string  `json:"timestamp_field"`
	EventTypeField string  `json:"event_type_field"`
	ActorIDField   *string `json:"actor_id_field"`
	ActorTypeField *string `json:"actor_type_field"`
	DataField      *string `json:"data_field"`
	Limit          *int    `json:"limit"`
}

// EventRecord is a normalized event-log entry.
type EventRecord struct {
	Timestamp *string         `json:"timestamp,omitempty"`
	EventType string          `json:"event_type"`
	ActorID   *string         `json:"actor_id,omitempty"`
	ActorType *string         `json:"actor_type,omitempty"`
	Data      json.RawMessage `json:"data,omitempty"`
}

// EventSummary reports aggregate counts for a projected event stream.
type EventSummary struct {
	TotalEvents     int            `json:"total_events"`
	ByEventType     map[string]int `json:"by_event_type"`
	ByActorID       map[string]int `json:"by_actor_id"`
	ByActorType     map[string]int `json:"by_actor_type"`
	OldestTimestamp *string        `json:"oldest_timestamp,omitempty"`
	NewestTimestamp *string        `json:"newest_timestamp,omitempty"`
}

// ProjectEventlog projects NDJSON/JSONL event records into normalized entries.
func ProjectEventlog(data []byte, config *EventProjectionConfig) ([]EventRecord, error) {
	raw, err := projectEventlog(data, encodeEventProjectionConfig(config))
	if err != nil {
		return nil, err
	}
	if len(raw) == 0 {
		return nil, nil
	}

	var events []EventRecord
	if err := json.Unmarshal(raw, &events); err != nil {
		return nil, &Error{Code: ErrFailed, Message: err.Error()}
	}
	return events, nil
}

// SummarizeEventlog summarizes NDJSON/JSONL event records.
func SummarizeEventlog(data []byte, config *EventProjectionConfig) (EventSummary, error) {
	raw, err := summarizeEventlog(data, encodeEventProjectionConfig(config))
	if err != nil {
		return EventSummary{}, err
	}
	if len(raw) == 0 {
		return EventSummary{}, nil
	}

	var summary EventSummary
	if err := json.Unmarshal(raw, &summary); err != nil {
		return EventSummary{}, &Error{Code: ErrFailed, Message: err.Error()}
	}
	return summary, nil
}

func encodeEventProjectionConfig(config *EventProjectionConfig) []byte {
	if config == nil {
		return nil
	}

	payload := eventProjectionConfigPayload{
		TimestampField: "timestamp",
		EventTypeField: "event_type",
	}

	actorID := "agent_id"
	payload.ActorIDField = &actorID
	actorType := "agent_type"
	payload.ActorTypeField = &actorType

	if config.TimestampField != "" {
		payload.TimestampField = config.TimestampField
	}
	if config.EventTypeField != "" {
		payload.EventTypeField = config.EventTypeField
	}
	if config.DisableActorID {
		payload.ActorIDField = nil
	} else if config.ActorIDField != "" {
		actorID = config.ActorIDField
		payload.ActorIDField = &actorID
	}
	if config.DisableActorType {
		payload.ActorTypeField = nil
	} else if config.ActorTypeField != "" {
		actorType = config.ActorTypeField
		payload.ActorTypeField = &actorType
	}
	if config.DataField != "" {
		dataField := config.DataField
		payload.DataField = &dataField
	}
	if config.Limit > 0 {
		limit := config.Limit
		payload.Limit = &limit
	}

	data, err := json.Marshal(payload)
	if err != nil {
		return nil
	}
	return data
}
