---
allowed-tools: Read, Bash(cat:*), Bash(head:*), Bash(ls:*)
argument-hint: <content or @file> [readme|blog|docs|pitch]
description: Transform technical content into engaging prose
model: sonnet
---

# Content Enhancement Request

Use the tech-writer sub-agent to transform the following technical content into compelling, factual prose.

## Source Material to Transform

$ARGUMENTS

## Transformation Context

Analyze the content and arguments provided above to determine:
- If a specific format was requested (readme, blog, docs, pitch), optimize for that
- If no format specified, infer from content type and optimize accordingly

### Format-Specific Guidelines

**If README format:**
- Focus on immediate clarity and value proposition
- Keep paragraphs short and scannable
- Emphasize what the tool does and how to use it
- Developer audience, technical but accessible

**If BLOG format:**
- Create narrative flow with engaging opening
- Build story arc from problem to solution
- Include context and background
- Balance technical depth with readability

**If DOCS format:**
- Comprehensive and organized
- Clear sections and hierarchy
- Searchable and reference-friendly
- Focus on completeness over engagement

**If PITCH format:**
- Emphasize business value and outcomes
- Lead with impressive metrics
- Focus on differentiation
- Executive-friendly language

## Additional Context

### Project Style Guide
!`if [ -f "CLAUDE.md" ]; then echo "Project guidelines found:"; head -20 CLAUDE.md 2>/dev/null | grep -i "style\|writing\|tone" || echo "No specific style guidelines"; else echo "No CLAUDE.md file"; fi`

### Content Type Detection
!`echo "$ARGUMENTS" | head -1 | cut -c1-100`

## Requirements for Transformation

1. **Maintain 100% factual accuracy** - Every claim must trace to source material
2. **No invented scenarios** - Don't add "imagine if" or "last week I" stories  
3. **No marketing fluff** - Avoid all banned words (powerful, seamless, revolutionary, etc.)
4. **Natural flow** - Vary sentence length and structure for rhythm
5. **Compelling lead** - Start with the most interesting fact
6. **Specific metrics** - Use exact numbers, not vague claims

## Expected Output

Transform the source material into prose that is:
- Impossible to skim (engaging)
- Impossible to forget (memorable)
- Impossible not to share (compelling)
- Completely accurate (factual)

Focus on finding the natural drama in the specifications rather than adding embellishments.