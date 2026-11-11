use rusqlite::Connection;

use crate::error::AppError;
use crate::models::{
    article, education, experience, faq, gaps, instructions, profile, project, skill, values,
};

use super::opaque_internal;
use super::prompt_templates::*;
use super::prompts::FIT_WRAPPER_TEMPLATE;

/// Build the base system prompt (shared by chat and fit) from all data in the
/// database. `build_system_prompt` appends the chat-only length cap after it.
/// Section order:
///  1. Identity (header — third person representative)
///  2. Elevator pitch + career narrative
///  3. Security block (anti-injection)
///  4. Core directive (brutal honesty + anti-hallucination)
///  5. Voice direction
///  6. Custom instructions
///  7. About
///  8. Work experience (public fields plus the private ones not dropped by #4713)
///  9. Skills self-assessment
/// 10. Explicit gaps & weaknesses
/// 11. Education
/// 12. Values & culture fit
/// 13. FAQ (verbatim-first)
/// 14. Projects (published only)
/// 15. Articles (published only)
/// 16. Compensation deflection
/// 17. Response guidelines
///
/// Returns the raw `rusqlite::Error`; the public builders map it to the opaque
/// AI error so SQLite text never reaches a response body (#4711 F4).
fn build_base_prompt(conn: &Connection) -> Result<String, rusqlite::Error> {
    let prof = profile::get_full(conn)?;
    let instrs = instructions::list_all(conn)?;
    let experiences = experience::list_all(conn)?;
    let skills = skill::list_all(conn)?;
    let edu = education::list(conn)?;
    let gaps_list = gaps::list_all(conn)?;
    let vals = values::get(conn)?;
    let faqs = faq::list_all(conn)?;
    let projects = project::list_all(conn)?;
    let articles = article::list_all(conn)?;

    let mut prompt = String::new();

    // --- 1. Identity header ---
    prompt.push_str(
        &HEADER_TEMPLATE
            .replace("{name}", &prof.name)
            .replace("{title}", &prof.title),
    );

    // --- 2. Elevator pitch + career narrative ---
    // Prefer pitch_long (the richer biographical version) for AI context;
    // fall back to pitch_short if pitch_long is empty so we never lose the
    // pitch entirely. This is per spec note 4: "Rune picks pitch_long as the
    // replacement (richer context) unless there's a reason not to."
    let pitch_for_ai = if !prof.pitch_long.is_empty() {
        &prof.pitch_long
    } else {
        &prof.pitch_short
    };
    if !pitch_for_ai.is_empty() {
        prompt.push_str(pitch_for_ai);
        prompt.push_str("\n\n");
    }
    if !prof.career_narrative.is_empty() {
        prompt.push_str(&prof.career_narrative);
        prompt.push_str("\n\n");
    }

    // --- 3. Security block ---
    prompt.push_str(SECURITY_BLOCK);

    // --- 4. Core directive ---
    prompt.push_str(&CORE_DIRECTIVE.replace("{name}", &prof.name));

    // --- 5. Voice direction ---
    prompt.push_str(&VOICE_DIRECTION.replace("{name}", &prof.name));

    // --- 6. Custom instructions ---
    prompt.push_str(SECTION_CUSTOM_INSTRUCTIONS);
    for instr in &instrs {
        prompt.push_str(&format!(
            "- [{}] {}\n",
            instr.instruction_type, instr.instruction
        ));
    }

    // --- 7. About ---
    prompt.push_str(SECTION_ABOUT);
    prompt.push_str(&format!("Looking for: {}\n", prof.looking_for));
    prompt.push_str(&format!("Not looking for: {}\n", prof.not_looking_for));

    let target_titles = if let Some(arr) = prof.target_titles.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        prof.target_titles.to_string()
    };
    prompt.push_str(&format!("Target titles: {}\n", target_titles));

    let target_stages = if let Some(arr) = prof.target_company_stages.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        prof.target_company_stages.to_string()
    };
    prompt.push_str(&format!("Target company stages: {}\n", target_stages));
    prompt.push_str(&format!("Management style: {}\n", prof.management_style));
    prompt.push_str(&format!("Work style: {}\n", prof.work_style));

    // --- 8. Work experience ---
    prompt.push_str(SECTION_EXPERIENCE);
    for exp in &experiences {
        let date_range = match &exp.end_date {
            Some(end) => format!("{} – {}", exp.start_date, end),
            None if exp.is_current => format!("{} – Present", exp.start_date),
            None => exp.start_date.clone(),
        };
        prompt.push_str(&format!(
            "\n### {} — {} ({})\n",
            exp.company_name, exp.title, date_range
        ));
        if !exp.location.is_empty() {
            prompt.push_str(&format!("Location: {}\n", exp.location));
        }
        if !exp.summary.is_empty() {
            prompt.push_str(&format!("{}\n", exp.summary));
        }

        // Public bullet points
        if let Some(bullets) = exp.bullet_points.as_array()
            && !bullets.is_empty()
        {
            prompt.push_str("Highlights:\n");
            for bullet in bullets {
                if let Some(b) = bullet.as_str() {
                    prompt.push_str(&format!("- {}\n", b));
                }
            }
        }

        // #4713: `why_joined`, `why_left` and `reports_would_say` are
        // intentionally NOT interpolated (same reasoning as the LLM07(a)
        // notes below). The fields that remain in the prompt carry no
        // withhold-instruction, so treat them as publicly extractable.
        if !exp.actual_contributions.is_empty() {
            prompt.push_str(&format!(
                "Actual contributions: {}\n",
                exp.actual_contributions
            ));
        }
        if !exp.proudest_achievement.is_empty() {
            prompt.push_str(&format!(
                "Proudest achievement: {}\n",
                exp.proudest_achievement
            ));
        }
        // LLM07(a) / R3: `would_do_differently` is intentionally NOT
        // interpolated. Prompt-only confidentiality is bypassable by
        // adversarial phrasing; the most-sensitive private fields are
        // dropped from the construction path entirely. The DB column,
        // struct field, and admin CRUD are unaffected — only this read
        // is removed.
        if !exp.challenges_faced.is_empty() {
            prompt.push_str(&format!("Challenges faced: {}\n", exp.challenges_faced));
        }
        if !exp.lessons_learned.is_empty() {
            prompt.push_str(&format!("Lessons learned: {}\n", exp.lessons_learned));
        }
        // LLM07(a) / R3: `manager_would_say` is intentionally NOT
        // interpolated (see the `would_do_differently` note above).
        if !exp.title_progression.is_empty() {
            prompt.push_str(&format!("Title progression: {}\n", exp.title_progression));
        }
        if !exp.quantified_impact.is_null() && exp.quantified_impact != serde_json::json!({}) {
            prompt.push_str(&format!("Quantified impact: {}\n", exp.quantified_impact));
        }
    }

    // --- 9. Skills self-assessment ---
    prompt.push_str(SECTION_SKILLS);

    let strong: Vec<_> = skills.iter().filter(|s| s.category == "strong").collect();
    let moderate: Vec<_> = skills.iter().filter(|s| s.category == "moderate").collect();
    let gap: Vec<_> = skills.iter().filter(|s| s.category == "gap").collect();

    let format_skill = |s: &&crate::models::skill::SkillFull| -> String {
        let mut line = format!(
            "- {} ({} yrs, {}/5)",
            s.skill_name, s.years_experience, s.self_rating
        );
        if !s.evidence.is_empty() {
            line.push_str(&format!(". Evidence: {}", s.evidence));
        }
        // LLM07(a) / R3: `honest_notes` is intentionally NOT interpolated.
        // It is the most-sensitive skill field; dropped from the
        // construction path entirely (column/struct/admin CRUD unaffected).
        line.push('\n');
        line
    };

    if !strong.is_empty() {
        prompt.push_str(SECTION_SKILLS_STRONG);
        for s in &strong {
            prompt.push_str(&format_skill(s));
        }
    }
    if !moderate.is_empty() {
        prompt.push_str(SECTION_SKILLS_MODERATE);
        for s in &moderate {
            prompt.push_str(&format_skill(s));
        }
    }
    if !gap.is_empty() {
        prompt.push_str(SECTION_SKILLS_GAPS);
        for s in &gap {
            prompt.push_str(&format_skill(s));
        }
    }

    // --- 10. Explicit gaps & weaknesses ---
    prompt.push_str(SECTION_GAPS);
    for g in &gaps_list {
        let interest = if g.interest_in_learning { "Yes" } else { "No" };
        prompt.push_str(&format!(
            "- [{}] {}: {} (Interested in learning: {})\n",
            g.gap_type, g.description, g.why_its_a_gap, interest
        ));
    }

    // --- 11. Education ---
    prompt.push_str(SECTION_EDUCATION);
    for e in &edu {
        prompt.push_str(&format!(
            "- {} — {}, {} ({} – {})\n",
            e.degree, e.institution, e.location, e.start_year, e.end_year
        ));
    }

    // --- 12. Values & culture fit ---
    prompt.push_str(SECTION_VALUES);
    if !vals.must_haves.is_empty() {
        prompt.push_str(&format!("Must-haves: {}\n", vals.must_haves));
    }
    if !vals.dealbreakers.is_empty() {
        prompt.push_str(&format!("Dealbreakers: {}\n", vals.dealbreakers));
    }
    if !vals.management_style_preferences.is_empty() {
        prompt.push_str(&format!(
            "Management style preferences: {}\n",
            vals.management_style_preferences
        ));
    }
    if !vals.team_size_preferences.is_empty() {
        prompt.push_str(&format!(
            "Team size preferences: {}\n",
            vals.team_size_preferences
        ));
    }
    if !vals.how_handle_conflict.is_empty() {
        prompt.push_str(&format!(
            "How he handles conflict: {}\n",
            vals.how_handle_conflict
        ));
    }
    if !vals.how_handle_ambiguity.is_empty() {
        prompt.push_str(&format!(
            "How he handles ambiguity: {}\n",
            vals.how_handle_ambiguity
        ));
    }
    if !vals.how_handle_failure.is_empty() {
        prompt.push_str(&format!(
            "How he handles failure: {}\n",
            vals.how_handle_failure
        ));
    }

    // --- 13. FAQ (verbatim-first) ---
    prompt.push_str(SECTION_FAQ);
    for f in &faqs {
        prompt.push_str(&format!("Q: {}\nA: {}\n\n", f.question, f.answer));
    }

    // --- 14. Projects (published only) ---
    let published_projects: Vec<_> = projects.iter().filter(|p| p.published).collect();
    if !published_projects.is_empty() {
        prompt.push_str(SECTION_PROJECTS);
        for p in &published_projects {
            let tech = if let Some(arr) = p.tech_stack.as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                p.tech_stack.to_string()
            };
            prompt.push_str(&format!("### {}\n", p.title));
            if !p.summary.is_empty() {
                prompt.push_str(&format!("{}\n", p.summary));
            }
            if !tech.is_empty() {
                prompt.push_str(&format!("Tech: {}\n", tech));
            }
            if !p.description.is_empty() {
                prompt.push_str(&format!("{}\n", p.description));
            }
            prompt.push('\n');
        }
    }

    // --- 15. Articles (published only) ---
    let published_articles: Vec<_> = articles.iter().filter(|a| a.published).collect();
    if !published_articles.is_empty() {
        prompt.push_str(SECTION_ARTICLES);
        for a in &published_articles {
            prompt.push_str(&format!("### {}\n", a.title));
            if !a.summary.is_empty() {
                prompt.push_str(&format!("{}\n", a.summary));
            }
            if !a.content.is_empty() {
                prompt.push_str(&format!("{}\n", a.content));
            }
            prompt.push('\n');
        }
    }

    // --- 16. Compensation deflection (no salary numbers) ---
    prompt.push_str(&COMPENSATION_DEFLECTION.replace("{name}", &prof.name));

    // --- 17. Response guidelines ---
    prompt.push_str(RESPONSE_GUIDELINES);

    Ok(prompt)
}

/// Build the system prompt for chat: the shared base prompt plus the chat-only
/// answer-length cap (#4656). Appended in code, after the DB-sourced sections,
/// so it comes after any contradicting `ai_instructions` row (placement, not
/// enforcement: the model still weighs both). Fit must not use this.
pub fn build_system_prompt(conn: &Connection) -> Result<String, AppError> {
    let mut prompt =
        build_base_prompt(conn).map_err(|e| opaque_internal("chat prompt: sqlite", e))?;
    prompt.push_str(CHAT_LENGTH_LIMIT);
    Ok(prompt)
}

/// Build the system prompt for fit analysis: base prompt + FIT_WRAPPER.
/// Loads the profile to get the candidate name for the wrapper template, then
/// calls `build_base_prompt` to produce the shared base prompt (which also loads
/// the profile internally). The wrapper is appended after the base prompt; the
/// chat-only length cap is not part of it.
pub fn build_fit_prompt(conn: &Connection) -> Result<String, AppError> {
    let prof = profile::get_full(conn).map_err(|e| opaque_internal("fit prompt: sqlite", e))?;
    let base = build_base_prompt(conn).map_err(|e| opaque_internal("fit prompt: sqlite", e))?;
    let wrapper = FIT_WRAPPER_TEMPLATE.replace("{name}", &prof.name);
    Ok(format!("{}\n\n{}", base, wrapper))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        run_migrations(&conn).expect("migrations");
        conn
    }

    fn seed_profile(conn: &Connection) {
        conn.execute(
            "UPDATE candidate_profile SET
                name = 'Jane Doe',
                title = 'Senior Engineer',
                pitch_short = 'I build reliable systems.',
                pitch_long = 'I build reliable systems.',
                career_narrative = 'Ten years of distributed systems work.',
                looking_for = 'IC or staff-level roles',
                not_looking_for = 'Management-only roles',
                target_titles = '[\"Staff Engineer\",\"Principal Engineer\"]',
                target_company_stages = '[\"Series B\",\"Series C\"]',
                management_style = 'Collaborative',
                work_style = 'Deep work blocks',
                salary_min = 200000,
                salary_max = 280000
             WHERE id = 1",
            [],
        )
        .unwrap();
    }

    fn seed_experience(conn: &Connection) {
        conn.execute(
            "INSERT INTO experiences (
                company_name, title, start_date, end_date, is_current,
                why_joined, why_left, actual_contributions, proudest_achievement,
                bullet_points, display_order
             ) VALUES (
                'Acme Corp', 'Lead Engineer', '2020-01', '2023-06', 0,
                'Loved the mission', 'Acquired by BigCo', 'Rebuilt the data pipeline',
                'Cut latency by 40%',
                '[\"Led team of 5\",\"Shipped 3 major features\"]',
                1
             )",
            [],
        )
        .unwrap();
    }

    fn seed_skills(conn: &Connection) {
        conn.execute(
            "INSERT INTO skills (skill_name, category, years_experience, self_rating, evidence, honest_notes)
             VALUES ('Rust', 'strong', 3, 4, 'Rewrote core service', 'Still learning async patterns')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO skills (skill_name, category, years_experience, self_rating, evidence, honest_notes)
             VALUES ('Go', 'moderate', 2, 3, 'Side projects', 'Not production experience')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO skills (skill_name, category, years_experience, self_rating, evidence, honest_notes)
             VALUES ('ML/AI', 'gap', 0, 1, '', 'No formal training')",
            [],
        )
        .unwrap();
    }

    fn seed_gaps(conn: &Connection) {
        conn.execute(
            "INSERT INTO gaps_weaknesses (gap_type, description, why_its_a_gap, interest_in_learning)
             VALUES ('skill', 'Formal ML experience', 'Never worked in ML team', 1)",
            [],
        )
        .unwrap();
    }

    fn seed_values(conn: &Connection) {
        conn.execute(
            "UPDATE values_culture SET
                must_haves = 'Autonomy and trust',
                dealbreakers = 'Micromanagement',
                management_style_preferences = 'Servant leadership',
                team_size_preferences = '5-15 people',
                how_handle_conflict = 'Direct conversation first',
                how_handle_ambiguity = 'Clarify then act',
                how_handle_failure = 'Blameless retrospective'
             WHERE id = 1",
            [],
        )
        .unwrap();
    }

    fn seed_faq(conn: &Connection) {
        conn.execute(
            "INSERT INTO faq_responses (question, answer, is_common_question)
             VALUES ('Why are you looking?', 'Seeking new challenges.', 1)",
            [],
        )
        .unwrap();
    }

    fn seed_instructions(conn: &Connection) {
        conn.execute(
            "INSERT INTO ai_instructions (instruction_type, instruction, priority)
             VALUES ('tone', 'Be concise and direct.', 10)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ai_instructions (instruction_type, instruction, priority)
             VALUES ('honesty', 'Never oversell skills.', 20)",
            [],
        )
        .unwrap();
    }

    fn seed_projects(conn: &Connection) {
        conn.execute(
            "INSERT INTO projects (title, slug, summary, description, tech_stack, url, sort_order, published)
             VALUES ('Published Project', 'pub-proj', 'A great project', 'Full details here', '[\"Rust\",\"Axum\"]', 'https://example.com', 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects (title, slug, summary, description, tech_stack, url, sort_order, published)
             VALUES ('Draft Project', 'draft-proj', 'Not ready', 'Hidden details', '[]', '', 2, 0)",
            [],
        )
        .unwrap();
    }

    fn seed_articles(conn: &Connection) {
        conn.execute(
            "INSERT INTO articles (title, slug, summary, content, tags, published_at, published)
             VALUES ('Published Article', 'pub-art', 'Article summary', 'Article content', '[]', '2024-01-01', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO articles (title, slug, summary, content, tags, published_at, published)
             VALUES ('Draft Article', 'draft-art', 'Hidden', 'Hidden content', '[]', NULL, 0)",
            [],
        )
        .unwrap();
    }

    fn seed_all(conn: &Connection) {
        seed_profile(conn);
        seed_experience(conn);
        seed_skills(conn);
        seed_gaps(conn);
        seed_values(conn);
        seed_faq(conn);
        seed_instructions(conn);
        seed_projects(conn);
        seed_articles(conn);
    }

    #[test]
    fn test_prompt_contains_profile_name_and_narrative() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Jane Doe"),
            "prompt must contain profile name"
        );
        assert!(
            prompt.contains("Ten years of distributed systems work."),
            "prompt must contain career_narrative"
        );
        assert!(
            prompt.contains("I build reliable systems."),
            "prompt must contain elevator_pitch"
        );
    }

    #[test]
    fn test_prompt_contains_experience_without_dropped_private_fields() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Acme Corp"),
            "prompt must contain company name"
        );
        assert!(
            !prompt.contains("Loved the mission"),
            "prompt must not contain why_joined (#4713)"
        );
        assert!(
            !prompt.contains("Acquired by BigCo"),
            "prompt must not contain why_left (#4713)"
        );
        assert!(
            prompt.contains("Rebuilt the data pipeline"),
            "prompt must contain actual_contributions"
        );
        assert!(
            prompt.contains("Cut latency by 40%"),
            "prompt must contain proudest_achievement"
        );
    }

    #[test]
    fn test_prompt_contains_skills_grouped_by_category() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("### Strong"),
            "prompt must have Strong section"
        );
        assert!(prompt.contains("Rust"), "prompt must contain Rust skill");
        assert!(
            prompt.contains("### Moderate"),
            "prompt must have Moderate section"
        );
        assert!(prompt.contains("Go"), "prompt must contain Go skill");
        assert!(prompt.contains("### Gaps"), "prompt must have Gaps section");
        assert!(
            prompt.contains("ML/AI"),
            "prompt must contain ML/AI gap skill"
        );
    }

    #[test]
    fn test_prompt_contains_gaps() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Formal ML experience"),
            "prompt must contain gap description"
        );
        assert!(
            prompt.contains("Never worked in ML team"),
            "prompt must contain why_its_a_gap"
        );
    }

    #[test]
    fn test_prompt_contains_values() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Autonomy and trust"),
            "must-haves in prompt"
        );
        assert!(prompt.contains("Micromanagement"), "dealbreakers in prompt");
        assert!(
            prompt.contains("Servant leadership"),
            "management prefs in prompt"
        );
        assert!(
            prompt.contains("Blameless retrospective"),
            "how_handle_failure in prompt"
        );
    }

    #[test]
    fn test_prompt_contains_faq() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Why are you looking?"),
            "FAQ question in prompt"
        );
        assert!(
            prompt.contains("Seeking new challenges."),
            "FAQ answer in prompt"
        );
    }

    #[test]
    fn test_prompt_contains_instructions_in_priority_order() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Be concise and direct."),
            "instruction 1 in prompt"
        );
        assert!(
            prompt.contains("Never oversell skills."),
            "instruction 2 in prompt"
        );
        // Higher priority (20) should appear before lower priority (10)
        let pos_honesty = prompt.find("Never oversell skills.").unwrap();
        let pos_tone = prompt.find("Be concise and direct.").unwrap();
        assert!(
            pos_honesty < pos_tone,
            "higher priority instruction (20) should appear before lower priority (10)"
        );
    }

    #[test]
    fn test_prompt_contains_published_projects_not_unpublished() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Published Project"),
            "published project in prompt"
        );
        assert!(
            !prompt.contains("Draft Project"),
            "draft project must NOT be in prompt"
        );
        assert!(
            !prompt.contains("Hidden details"),
            "draft project details must not appear"
        );
    }

    #[test]
    fn test_prompt_contains_published_articles_not_drafts() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Published Article"),
            "published article in prompt"
        );
        assert!(
            !prompt.contains("Draft Article"),
            "draft article must NOT be in prompt"
        );
        assert!(
            !prompt.contains("Hidden content"),
            "draft article content must not appear"
        );
    }

    // Decision 6: Salary removed — check for compensation deflection instead
    #[test]
    fn test_prompt_contains_compensation_deflection_not_salary_numbers() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("COMPENSATION"),
            "compensation section in prompt"
        );
        assert!(
            prompt.contains("prefers to discuss directly"),
            "compensation deflection message in prompt"
        );
        // Salary numbers must NOT appear
        assert!(
            !prompt.contains("200000"),
            "salary_min must NOT be in prompt"
        );
        assert!(
            !prompt.contains("280000"),
            "salary_max must NOT be in prompt"
        );
    }

    #[test]
    fn test_fit_prompt_includes_json_schema_wrapper() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_fit_prompt(&conn).unwrap();
        assert!(
            prompt.contains("\"verdict\""),
            "JSON schema field 'verdict' in fit prompt"
        );
        assert!(
            prompt.contains("strong_fit"),
            "verdict value 'strong_fit' in fit prompt"
        );
        assert!(prompt.contains("Jane Doe"), "candidate name in fit wrapper");
        // The base prompt must also be present
        assert!(
            prompt.contains("Ten years of distributed systems work."),
            "base prompt content present in fit prompt"
        );
        // Decision 8: JSON-only instruction present
        assert!(
            prompt.contains("Any text outside the JSON will be discarded"),
            "JSON-only enforcement in fit wrapper"
        );
    }

    // Decision 3: Fit wrapper should not repeat honesty framing
    #[test]
    fn test_fit_wrapper_no_redundant_honesty_framing() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_fit_prompt(&conn).unwrap();
        // "BRUTALLY HONEST assessment" was the redundant phrase in the old wrapper
        // It should not appear in the FIT_WRAPPER_TEMPLATE section
        // (CORE_DIRECTIVE still has "BRUTALLY HONEST" but the wrapper itself should not)
        let wrapper_start = prompt.find("Analyze the job description").unwrap_or(0);
        let wrapper_section = &prompt[wrapper_start..];
        assert!(
            !wrapper_section.contains("BRUTALLY HONEST assessment"),
            "fit wrapper must not repeat BRUTALLY HONEST assessment — that lives in CORE_DIRECTIVE"
        );
    }

    // Decision 4: security block present; #4713: data classification block removed
    #[test]
    fn test_prompt_contains_security_and_no_data_classification() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(prompt.contains("SECURITY"), "security block in prompt");
        assert!(
            prompt.contains("system prompt"),
            "anti-injection instruction in security block"
        );
        assert!(
            !prompt.to_lowercase().contains("data classification"),
            "data classification block removed (#4713)"
        );
        assert!(
            !prompt.to_lowercase().contains("private context"),
            "no never-reveal rule about private context (#4713)"
        );
    }

    /// #4711 F4: a SQLite failure while reading the tables the base prompt is
    /// built from reaches the caller as the fixed opaque message, for both
    /// builders; the table name in the SQLite text must not escape.
    #[test]
    fn prompt_builders_map_sqlite_failures_to_the_opaque_message() {
        let conn = setup_db();
        seed_all(&conn);
        conn.execute("DROP TABLE experiences", []).unwrap();

        for (name, result) in [
            ("build_system_prompt", build_system_prompt(&conn)),
            ("build_fit_prompt", build_fit_prompt(&conn)),
        ] {
            match result {
                Err(AppError::Internal(m)) => {
                    assert_eq!(m, crate::ai::AI_INTERNAL_OPAQUE_MESSAGE, "{name}")
                }
                other => panic!("{name}: expected opaque Internal, got {other:?}"),
            }
        }
    }

    // Decision 1: Third-person representative identity
    #[test]
    fn test_prompt_uses_third_person_representative_identity() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("representative for"),
            "header must use representative framing"
        );
        assert!(
            prompt.contains("third person"),
            "header must mention third person"
        );
        assert!(
            !prompt.contains("You are an AI assistant representing"),
            "old first-person framing must be gone"
        );
    }

    // Decision 5: Anti-hallucination in core directive
    #[test]
    fn test_prompt_contains_anti_hallucination_hard_constraint() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("HARD CONSTRAINT"),
            "anti-hallucination hard constraint in core directive"
        );
        assert!(
            prompt.contains("Never fabricate"),
            "fabrication prohibition in prompt"
        );
    }

    // Decision 4: FAQ verbatim-first instruction
    #[test]
    fn test_prompt_faq_has_verbatim_first_instruction() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("use the provided answer as your primary source"),
            "FAQ verbatim-first instruction in prompt"
        );
    }

    // #4656: long chat answers hit the 45 s cap. The length cap is chat-only;
    // fit shares the base prompt and must not receive it. Asserts on stable
    // fragments, not the whole sentence. Proves the text is sent, not that the
    // model obeys it.
    #[test]
    fn test_chat_prompt_caps_length_and_avoids_yes_no_follow_up() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.ends_with(CHAT_LENGTH_LIMIT),
            "length cap must be the tail of the chat prompt"
        );
        assert!(
            prompt.contains("700 words, even when"),
            "cap must hold even when the visitor asks for more detail"
        );
        assert!(
            prompt.contains("without memory of earlier ones"),
            "chat is stateless; follow-up must be a standalone question"
        );
    }

    #[test]
    fn test_fit_prompt_does_not_carry_chat_length_cap() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_fit_prompt(&conn).unwrap();
        assert!(
            !prompt.contains(CHAT_LENGTH_LIMIT),
            "fit prompt must not carry the chat length cap"
        );
        assert!(
            prompt.contains("Keep responses compressed unless detail is requested"),
            "fit keeps the shared response guidelines unchanged"
        );
        let wrapper = FIT_WRAPPER_TEMPLATE.replace("{name}", "Jane Doe");
        assert_eq!(
            prompt,
            format!("{}\n\n{}", build_base_prompt(&conn).unwrap(), wrapper),
            "fit prompt is exactly base + wrapper"
        );
    }

    // Spec 2026-07-24-experiences-visible-flag R-0003 tripwire: build_fit_prompt
    // no longer delegates to build_system_prompt, so the fit path carries its own
    // pin. Per the spec the AI surface receives experiences regardless of `visible`.
    #[test]
    fn test_fit_prompt_includes_visible_and_hidden_experiences() {
        let conn = setup_db();
        seed_all(&conn);
        for (name, visible) in [("ZZ-VISIBLE-Solstice-4a1c", 1), ("ZZ-HIDDEN-Umbra-9e2f", 0)] {
            conn.execute(
                "INSERT INTO experiences (
                    company_name, title, location, start_date, end_date, is_current,
                    summary, bullet_points, display_order, visible
                 ) VALUES (?1, 'Engineer', 'Remote', '2010-01', '2012-01', 0, 'S.', '[]', 50, ?2)",
                rusqlite::params![name, visible],
            )
            .unwrap();
        }
        let prompt = build_fit_prompt(&conn).unwrap();
        assert!(
            prompt.contains("ZZ-VISIBLE-Solstice-4a1c"),
            "visible row in fit prompt"
        );
        assert!(
            prompt.contains("ZZ-HIDDEN-Umbra-9e2f"),
            "hidden row in fit prompt"
        );
    }
}
