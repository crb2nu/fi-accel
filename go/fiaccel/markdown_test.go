package fiaccel

import (
	"encoding/json"
	"fmt"
	"testing"
)

func TestStripMarkdownCodeFence(t *testing.T) {
	input := "```json\n{\"ok\":true}\n```"
	if got := StripMarkdownCodeFence(input); got != "{\"ok\":true}" {
		t.Fatalf("StripMarkdownCodeFence() = %q", got)
	}
}

func TestStripMarkdownCodeFenceServicePayload(t *testing.T) {
	input := "\n```json\n{\"tool\":\"lookup\",\"arguments\":{\"q\":\"hello\"}}\r\n```\n"
	got := StripMarkdownCodeFence(input)

	var payload struct {
		Tool      string         `json:"tool"`
		Arguments map[string]any `json:"arguments"`
	}
	if err := json.Unmarshal([]byte(got), &payload); err != nil {
		t.Fatalf("stripped payload is not valid JSON: %v", err)
	}
	if payload.Tool != "lookup" || payload.Arguments["q"] != "hello" {
		t.Fatalf("unexpected payload: %+v", payload)
	}
}

func TestStripMarkdownCodeFenceKeepsNonWrappingContent(t *testing.T) {
	input := "Here is the payload:\n```json\n{\"ok\":true}\n```"
	if got := StripMarkdownCodeFence(input); got != input {
		t.Fatalf("StripMarkdownCodeFence() = %q, want original", got)
	}
}

func TestExtractMarkdownCodeBlocks(t *testing.T) {
	input := "Before\n```go\nfmt.Println(\"hi\")\n```\nAfter\n```json\n{\"ok\":true}\n```"
	blocks := ExtractMarkdownCodeBlocks(input)
	if len(blocks) != 2 {
		t.Fatalf("len(blocks) = %d, want 2", len(blocks))
	}
	if blocks[0].Language != "go" || blocks[0].Content != "fmt.Println(\"hi\")" {
		t.Fatalf("unexpected first block: %+v", blocks[0])
	}
	if blocks[1].Language != "json" || blocks[1].Content != "{\"ok\":true}" {
		t.Fatalf("unexpected second block: %+v", blocks[1])
	}
}

func TestExtractMarkdownCodeBlocksServiceTranscript(t *testing.T) {
	input := "Assistant:\n\n```json\n{\"status\":\"ok\"}\n```\n\nThen:\n```text\nstored\n```\n"
	blocks := ExtractMarkdownCodeBlocks(input)
	if len(blocks) != 2 {
		t.Fatalf("len(blocks) = %d, want 2", len(blocks))
	}
	if blocks[0].Language != "json" || blocks[0].Content != "{\"status\":\"ok\"}" {
		t.Fatalf("unexpected JSON block: %+v", blocks[0])
	}
	if blocks[1].Language != "text" || blocks[1].Content != "stored" {
		t.Fatalf("unexpected text block: %+v", blocks[1])
	}
}

func TestMarkdownToText(t *testing.T) {
	input := "---\n" +
		"title: Demo\n" +
		"---\n\n" +
		"# Heading\n\n" +
		"Read the [docs](https://example.com) and `run tests`.\n\n" +
		"```ts\n" +
		"console.log(\"hidden\")\n" +
		"```\n\n" +
		"- bullet one\n" +
		"> quoted\n"

	got := MarkdownToText(input)
	want := "Heading Read the docs and run tests. bullet one quoted"
	if got != want {
		t.Fatalf("MarkdownToText() = %q, want %q", got, want)
	}
}

func TestMarkdownToTextServiceNote(t *testing.T) {
	input := "## Action\n\n" +
		"Store [the note](https://example.test/note) before routing.\n\n" +
		"```json\n" +
		"{\"internal\":true}\n" +
		"```\n\n" +
		"1. Preserve user-visible text\n" +
		"2. Drop fenced tool payloads\n"

	got := MarkdownToText(input)
	want := "Action Store the note before routing. Preserve user-visible text Drop fenced tool payloads"
	if got != want {
		t.Fatalf("MarkdownToText() = %q, want %q", got, want)
	}
}

func TestAnalyzeMarkdown(t *testing.T) {
	stats := AnalyzeMarkdown("# Hello\n\nThis is a short paragraph for testing.", 200)
	if stats.Text != "Hello This is a short paragraph for testing." {
		t.Fatalf("Text = %q", stats.Text)
	}
	if stats.WordCount != 8 {
		t.Fatalf("WordCount = %d", stats.WordCount)
	}
	if stats.EstimatedTokens != 10 {
		t.Fatalf("EstimatedTokens = %d", stats.EstimatedTokens)
	}
	if stats.ReadingTimeMinutes != 1 {
		t.Fatalf("ReadingTimeMinutes = %d", stats.ReadingTimeMinutes)
	}
}

func TestMarkdownHelpersCapability(t *testing.T) {
	if !RuntimeCapabilities().MarkdownHelpers {
		t.Fatalf("expected markdown helpers to be available")
	}
}

func ExampleStripMarkdownCodeFence() {
	payload := fiaccelModelOutput()
	clean := StripMarkdownCodeFence(payload)
	fmt.Println(clean)
	// Output:
	// {"ok":true}
}

func ExampleMarkdownToText() {
	note := "# Dispatch\n\nRoute [the request](https://example.test) after `validation`."
	fmt.Println(MarkdownToText(note))
	// Output:
	// Dispatch Route the request after validation.
}

func ExampleAnalyzeMarkdown() {
	stats := AnalyzeMarkdown("A short operational note.", 0)
	fmt.Println(stats.WordCount, stats.EstimatedTokens, stats.ReadingTimeMinutes)
	// Output:
	// 4 5 1
}

func fiaccelModelOutput() string {
	return "```json\n{\"ok\":true}\n```"
}
