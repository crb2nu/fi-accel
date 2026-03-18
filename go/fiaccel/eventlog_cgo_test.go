//go:build cgo

package fiaccel

import "testing"

func TestProjectEventlog(t *testing.T) {
	events, err := ProjectEventlog(
		[]byte("{\"timestamp\":\"2026-03-18T10:00:00Z\",\"status\":\"success\",\"entity_id\":\"abc-123\",\"tool\":\"workflow.advance\"}\n"),
		&EventProjectionConfig{
			EventTypeField:   "status",
			ActorIDField:     "entity_id",
			DisableActorType: true,
		},
	)
	if err != nil {
		t.Fatalf("ProjectEventlog: %v", err)
	}
	if len(events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(events))
	}
	if events[0].ActorID == nil || *events[0].ActorID != "abc-123" {
		t.Fatalf("unexpected actor id: %+v", events[0])
	}
	if events[0].EventType != "success" {
		t.Fatalf("unexpected event type: %+v", events[0])
	}
	if string(events[0].Data) == "" {
		t.Fatalf("expected payload data: %+v", events[0])
	}
}

func TestSummarizeEventlog(t *testing.T) {
	summary, err := SummarizeEventlog(
		[]byte("{\"timestamp\":\"2026-03-18T10:00:00Z\",\"event_type\":\"started\",\"agent_id\":\"codex\"}\n{\"timestamp\":\"2026-03-18T10:01:00Z\",\"event_type\":\"started\",\"agent_id\":\"codex\"}\n"),
		nil,
	)
	if err != nil {
		t.Fatalf("SummarizeEventlog: %v", err)
	}
	if summary.TotalEvents != 2 {
		t.Fatalf("unexpected total events: %+v", summary)
	}
	if summary.ByEventType["started"] != 2 {
		t.Fatalf("unexpected by_event_type: %+v", summary.ByEventType)
	}
	if summary.ByActorID["codex"] != 2 {
		t.Fatalf("unexpected by_actor_id: %+v", summary.ByActorID)
	}
}
