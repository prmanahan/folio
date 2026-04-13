use rusqlite::Connection;

use crate::error::AppError;
use crate::models::{
    article, education, experience, faq, gaps, instructions, profile, project, skill, values,
};

use super::prompt_templates::*;
use super::prompts::FIT_WRAPPER_TEMPLATE;

/// Build the full system prompt from all data in the database.
/// Section order:
///  1. Identity (header — third person representative)
///  2. Elevator pitch + career narrative
///  3. Security block (anti-injection)
///  4. Core directive (brutal honesty + anti-hallucination)
///  5. Voice direction
///  6. Data classification rules
///  7. Custom instructions
///  8. About
///  9. Work experience (with private fields)
/// 10. Skills self-assessment
/// 11. Explicit gaps & weaknesses
/// 12. Education
/// 13. Values & culture fit
/// 14. FAQ (verbatim-first)
/// 15. Projects (published only)
/// 16. Articles (published only)
/// 17. Compensation deflection
/// 18. Response guidelines
pub fn build_system_prompt(conn: &Connection) -> Result<String, AppError> {
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
    if !prof.elevator_pitch.is_empty() {
        prompt.push_str(&prof.elevator_pitch);
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

    // --- 6. Data classification rules ---
    prompt.push_str(DATA_CLASSIFICATION);

    // --- 7. Custom instructions ---
    prompt.push_str(SECTION_CUSTOM_INSTRUCTIONS);
    for instr in &instrs {
        prompt.push_str(&format!(
            "- [{}] {}\n",
            instr.instruction_type, instr.instruction
        ));
    }

    // --- 8. About ---
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

    // --- 9. Work experience ---
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

        // Private context fields (for AI context only — see DATA_CLASSIFICATION)
        if !exp.why_joined.is_empty() {
            prompt.push_str(&format!("Why joined: {}\n", exp.why_joined));
        }
        if !exp.why_left.is_empty() {
            prompt.push_str(&format!("Why left: {}\n", exp.why_left));
        }
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
        if !exp.would_do_differently.is_empty() {
            prompt.push_str(&format!(
                "Would do differently: {}\n",
                exp.would_do_differently
            ));
        }
        if !exp.challenges_faced.is_empty() {
            prompt.push_str(&format!("Challenges faced: {}\n", exp.challenges_faced));
        }
        if !exp.lessons_learned.is_empty() {
            prompt.push_str(&format!("Lessons learned: {}\n", exp.lessons_learned));
        }
        if !exp.manager_would_say.is_empty() {
            prompt.push_str(&format!("Manager would say: {}\n", exp.manager_would_say));
        }
        if !exp.reports_would_say.is_empty() {
            prompt.push_str(&format!("Reports would say: {}\n", exp.reports_would_say));
        }
        if !exp.title_progression.is_empty() {
            prompt.push_str(&format!("Title progression: {}\n", exp.title_progression));
        }
        if !exp.quantified_impact.is_null() && exp.quantified_impact != serde_json::json!({}) {
            prompt.push_str(&format!("Quantified impact: {}\n", exp.quantified_impact));
        }
    }

    // --- 10. Skills self-assessment ---
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
        if !s.honest_notes.is_empty() {
            line.push_str(&format!(". Notes: {}", s.honest_notes));
        }
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

    // --- 11. Explicit gaps & weaknesses ---
    prompt.push_str(SECTION_GAPS);
    for g in &gaps_list {
        let interest = if g.interest_in_learning { "Yes" } else { "No" };
        prompt.push_str(&format!(
            "- [{}] {}: {} (Interested in learning: {})\n",
            g.gap_type, g.description, g.why_its_a_gap, interest
        ));
    }

    // --- 12. Education ---
    prompt.push_str(SECTION_EDUCATION);
    for e in &edu {
        prompt.push_str(&format!(
            "- {} — {}, {} ({} – {})\n",
            e.degree, e.institution, e.location, e.start_year, e.end_year
        ));
    }

    // --- 13. Values & culture fit ---
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

    // --- 14. FAQ (verbatim-first) ---
    prompt.push_str(SECTION_FAQ);
    for f in &faqs {
        prompt.push_str(&format!("Q: {}\nA: {}\n\n", f.question, f.answer));
    }

    // --- 15. Projects (published only) ---
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

    // --- 16. Articles (published only) ---
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

    // --- 17. Compensation deflection (no salary numbers) ---
    prompt.push_str(&COMPENSATION_DEFLECTION.replace("{name}", &prof.name));

    // --- 18. Response guidelines ---
    prompt.push_str(RESPONSE_GUIDELINES);

    Ok(prompt)
}

/// Build the system prompt for fit analysis: base prompt + FIT_WRAPPER.
/// Loads the profile to get the candidate name for the wrapper template, then
/// calls `build_system_prompt` to produce the full base prompt (which also loads
/// the profile internally). The wrapper is appended after the base prompt.
pub fn build_fit_prompt(conn: &Connection) -> Result<String, AppError> {
    let prof = profile::get_full(conn)?;
    let base = build_system_prompt(conn)?;
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
                elevator_pitch = 'I build reliable systems.',
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
    fn test_prompt_contains_experience_with_private_fields() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(
            prompt.contains("Acme Corp"),
            "prompt must contain company name"
        );
        assert!(
            prompt.contains("Loved the mission"),
            "prompt must contain why_joined"
        );
        assert!(
            prompt.contains("Acquired by BigCo"),
            "prompt must contain why_left"
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

    // Decision 4 + 7: Security and data classification sections present
    #[test]
    fn test_prompt_contains_security_and_data_classification() {
        let conn = setup_db();
        seed_all(&conn);
        let prompt = build_system_prompt(&conn).unwrap();
        assert!(prompt.contains("SECURITY"), "security block in prompt");
        assert!(
            prompt.contains("system prompt"),
            "anti-injection instruction in security block"
        );
        assert!(
            prompt.contains("DATA CLASSIFICATION"),
            "data classification block in prompt"
        );
        assert!(
            prompt.contains("for YOUR context only"),
            "private context disclosure rule in prompt"
        );
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
}
