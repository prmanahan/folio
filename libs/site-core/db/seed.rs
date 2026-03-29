use rusqlite::Connection;

/// Seed the database with fictional demo data for testing.
/// Uses the same "Alex Rivera" persona as data/seed.sql.
pub fn seed_test_data(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        UPDATE candidate_profile SET
            name = 'Alex Rivera',
            email = 'alex@example.com',
            title = 'Software Architect / Engineering Manager',
            location = 'Vancouver, BC',
            phone = '604-555-0199',
            elevator_pitch = 'Software architect with 12 years building distributed systems.',
            availability_status = 'open',
            remote_preference = 'remote'
        WHERE id = 1;

        INSERT INTO experiences (company_name, title, location, start_date, end_date, is_current, summary, bullet_points, display_order)
        VALUES ('Meridian Systems', 'Software Architect', 'Vancouver, BC', '2022-01', NULL, 1, 'Event processing platform.', '[\"Designed event-driven architecture\"]', 1);

        INSERT INTO skills (skill_name, category, years_experience, last_used)
        VALUES ('Java', 'strong', 12, '2024'),
               ('Rust', 'moderate', 2, '2026');

        INSERT INTO education (degree, institution, location, start_year, end_year)
        VALUES ('BSc Computer Science', 'University of British Columbia', 'Vancouver, BC', '2010', '2014');

        INSERT INTO projects (title, slug, summary, description, tech_stack, url, sort_order, published)
        VALUES ('EventFlow', 'eventflow', 'Event processing framework.', 'Built at Meridian.', '[\"Go\",\"Kafka\"]', '', 1, 1);

        INSERT INTO articles (title, slug, summary, content, tags, published_at, published)
        VALUES ('Event Sourcing', 'event-sourcing', 'Lessons learned.', '# Event Sourcing', '[\"architecture\"]', '2025-11-15', 1);

        INSERT INTO links (label, url, icon, sort_order)
        VALUES ('LinkedIn', 'https://linkedin.com/in/alex-rivera-example', 'linkedin', 1),
               ('GitHub', 'https://github.com/alex-rivera-example', 'github', 2),
               ('Email', 'mailto:alex@example.com', 'mail', 3);

        INSERT INTO faq_responses (question, answer, is_common_question)
        VALUES ('What are you looking for?', 'Architecture roles with technical ownership.', 1),
               ('Why leaving?', 'Looking for a new challenge.', 0);
        "
    )?;
    Ok(())
}
