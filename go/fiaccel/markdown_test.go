package fiaccel

import "testing"

func TestStripMarkdownCodeFence(t *testing.T) {
	input := "```json\n{\"ok\":true}\n```"
	if got := StripMarkdownCodeFence(input); got != "{\"ok\":true}" {
		t.Fatalf("StripMarkdownCodeFence() = %q", got)
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
