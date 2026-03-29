// Template for the main system prompt. Placeholders are replaced by build_system_prompt().
// The actual prompt is built programmatically in context.rs. The constants below are section
// delimiters and the fit-wrapper template that are used verbatim.

/// Wrapper appended to the system prompt for fit-analysis requests.
/// The placeholder `{name}` is replaced at build time.
/// Decision 3: Removed redundant "BRUTALLY HONEST" framing — that lives in CORE_DIRECTIVE.
/// Decision 8: Strengthened JSON-only instruction.
pub const FIT_WRAPPER_TEMPLATE: &str = r#"Analyze this job description to assess fit for {name}.

Instructions:
1. Compare JD requirements to {name}'s actual experience
2. Identify specific requirements {name} does not meet
3. Note what transfers even if it's not an exact match
4. Assess culture and work-style alignment

IMPORTANT: Your entire response must be a single JSON object. No preamble, no markdown fences, no explanation outside the JSON. All reasoning goes inside the JSON fields. Any text outside the JSON will be discarded.

The JSON MUST match this exact structure:
{
  "verdict": "strong_fit" or "worth_conversation" or "probably_not",
  "headline": "Brief 5-10 word headline",
  "opening": "1-2 sentence direct assessment",
  "gaps": [{"requirement": "What JD asks for", "gap_title": "Short gap title", "explanation": "Why this is a gap"}],
  "transfers": [{"skill": "What transfers", "relevance": "How it applies"}],
  "recommendation": "Direct advice for the hiring team"
}"#;
