package fiaccel

import (
	"regexp"
	"strings"
)

const defaultReadingWPM = 238

// MarkdownCodeBlock is a fenced triple-backtick code block extracted from Markdown.
type MarkdownCodeBlock struct {
	Language string `json:"language,omitempty"`
	Content  string `json:"content"`
}

// MarkdownTextStats reports compact plain text and lightweight document metrics.
type MarkdownTextStats struct {
	Text               string `json:"text"`
	WordCount          int    `json:"word_count"`
	EstimatedTokens    int    `json:"estimated_tokens"`
	ReadingTimeMinutes int    `json:"reading_time_minutes"`
}

type parsedMarkdownCodeBlock struct {
	MarkdownCodeBlock
	start int
	end   int
}

var (
	markdownImageRE         = regexp.MustCompile(`!\[([^\]]*)\]\([^)]+\)`)
	markdownLinkRE          = regexp.MustCompile(`\[([^\]]+)\]\([^)]+\)`)
	markdownAutolinkRE      = regexp.MustCompile(`<((?:https?|mailto):[^>]+)>`)
	markdownInlineCodeRE    = regexp.MustCompile("`([^`\n]+)`")
	markdownHeadingRE       = regexp.MustCompile(`(?m)^\s{0,3}#{1,6}\s*`)
	markdownBlockquoteRE    = regexp.MustCompile(`(?m)^\s*>\s?`)
	markdownUnorderedListRE = regexp.MustCompile(`(?m)^\s*[-*+]\s+`)
	markdownOrderedListRE   = regexp.MustCompile(`(?m)^\s*\d+\.\s+`)
	markdownHTMLTagRE       = regexp.MustCompile(`<[^>]+>`)
	markdownWhitespaceRE    = regexp.MustCompile(`\s+`)
)

// StripMarkdownCodeFence removes a single wrapping triple-backtick fence.
func StripMarkdownCodeFence(input string) string {
	trimmed := strings.TrimSpace(input)
	blocks := parseMarkdownCodeBlocks(trimmed)
	if len(blocks) != 1 {
		return trimmed
	}

	block := blocks[0]
	if strings.TrimSpace(trimmed[:block.start]) == "" && strings.TrimSpace(trimmed[block.end:]) == "" {
		return block.Content
	}

	return trimmed
}

// ExtractMarkdownCodeBlocks returns fenced triple-backtick code blocks in order.
func ExtractMarkdownCodeBlocks(input string) []MarkdownCodeBlock {
	parsed := parseMarkdownCodeBlocks(input)
	blocks := make([]MarkdownCodeBlock, 0, len(parsed))
	for _, block := range parsed {
		blocks = append(blocks, block.MarkdownCodeBlock)
	}
	return blocks
}

// MarkdownToText converts markdown into compact plain text.
func MarkdownToText(input string) string {
	text := stripMarkdownFrontmatter(input)
	text = removeMarkdownCodeBlocks(text)
	text = markdownImageRE.ReplaceAllString(text, "$1")
	text = markdownLinkRE.ReplaceAllString(text, "$1")
	text = markdownAutolinkRE.ReplaceAllString(text, "$1")
	text = markdownInlineCodeRE.ReplaceAllString(text, "$1")
	text = markdownHeadingRE.ReplaceAllString(text, "")
	text = markdownBlockquoteRE.ReplaceAllString(text, "")
	text = markdownUnorderedListRE.ReplaceAllString(text, "")
	text = markdownOrderedListRE.ReplaceAllString(text, "")
	text = markdownHTMLTagRE.ReplaceAllString(text, " ")

	for _, marker := range []string{"**", "__", "~~", "*", "_", "#", "|"} {
		text = strings.ReplaceAll(text, marker, " ")
	}

	return strings.TrimSpace(markdownWhitespaceRE.ReplaceAllString(text, " "))
}

// AnalyzeMarkdown computes plain text plus basic reading and token estimates.
func AnalyzeMarkdown(input string, wordsPerMinute int) MarkdownTextStats {
	if wordsPerMinute <= 0 {
		wordsPerMinute = defaultReadingWPM
	}

	text := MarkdownToText(input)
	wordCount := len(strings.Fields(text))
	estimatedTokens := int(float64(wordCount) * 1.3)
	readingTime := 0
	if wordCount > 0 {
		readingTime = (wordCount + wordsPerMinute - 1) / wordsPerMinute
		if readingTime < 1 {
			readingTime = 1
		}
	}

	return MarkdownTextStats{
		Text:               text,
		WordCount:          wordCount,
		EstimatedTokens:    estimatedTokens,
		ReadingTimeMinutes: readingTime,
	}
}

func parseMarkdownCodeBlocks(input string) []parsedMarkdownCodeBlock {
	var blocks []parsedMarkdownCodeBlock
	var open *parsedMarkdownCodeBlock
	offset := 0

	for _, segment := range strings.SplitAfter(input, "\n") {
		line := strings.TrimRight(segment, "\r\n")
		trimmed := strings.TrimLeft(line, " \t")

		if open == nil {
			if rest, ok := strings.CutPrefix(trimmed, "```"); ok {
				language := ""
				if fields := strings.Fields(rest); len(fields) > 0 {
					language = fields[0]
				}
				open = &parsedMarkdownCodeBlock{
					MarkdownCodeBlock: MarkdownCodeBlock{Language: language},
					start:             offset,
				}
			}
		} else if strings.HasPrefix(trimmed, "```") {
			open.end = offset + len(segment)
			open.Content = strings.TrimRight(open.Content, "\r\n")
			blocks = append(blocks, *open)
			open = nil
		} else {
			open.Content += segment
		}

		offset += len(segment)
	}

	return blocks
}

func stripMarkdownFrontmatter(input string) string {
	trimmed := strings.TrimPrefix(input, "\uFEFF")
	if !strings.HasPrefix(trimmed, "---\n") && !strings.HasPrefix(trimmed, "---\r\n") {
		return trimmed
	}

	offset := 0
	first := true
	for _, segment := range strings.SplitAfter(trimmed, "\n") {
		line := strings.TrimRight(segment, "\r\n")
		offset += len(segment)
		if first {
			first = false
			continue
		}
		if line == "---" || line == "..." {
			return trimmed[offset:]
		}
	}

	return trimmed
}

func removeMarkdownCodeBlocks(input string) string {
	blocks := parseMarkdownCodeBlocks(input)
	if len(blocks) == 0 {
		return input
	}

	var builder strings.Builder
	cursor := 0
	for _, block := range blocks {
		builder.WriteString(input[cursor:block.start])
		builder.WriteByte(' ')
		cursor = block.end
	}
	builder.WriteString(input[cursor:])
	return builder.String()
}
