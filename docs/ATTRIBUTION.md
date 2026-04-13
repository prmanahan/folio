# Attribution

## Database Schema & AI Architecture

The database schema (candidate_profile, experiences, skills, gaps_weaknesses, values_culture, faq_responses, ai_instructions), admin panel structure, and AI prompt architecture for this site were adapted from Nate Jones's "How to Build an AI-Powered Portfolio Site with Lovable" tutorial, available to subscribers of his Substack.

Nate's video walkthrough is available at: https://www.youtube.com/watch?v=your-video-id-here

**What was adapted:**
- 7-table portfolio database schema with public/private data separation
- "Brutal honesty" AI philosophy and core directive structure
- System prompt section layout (About, Experience, Skills, Gaps, Values, FAQ)
- JD analyzer concept (fit verdict with gaps/transfers/recommendation)
- Admin panel tab structure

**What is original work:**
- Rust/Axum backend (guide uses Lovable + Supabase)
- SvelteKit frontend
- Token-based authentication
- Third-person voice (guide uses first-person)
- Anti-hallucination constraints, data classification rules, security block
- Voice profile and compensation deflection
- Projects, Articles, Links, and Agents tables and pages
- Page hit tracking with privacy-preserving IP hashing
- Rate limiting
- `/job-fit` 8-dimension weighted scoring system
- The entire Puck agent orchestration system

## Related Projects

The OB1 project by Nate Jones (https://github.com/NateBJones-Projects/OB1, FSL-1.1-MIT license) is a broader persistent AI memory system that includes a job-hunt pipeline extension. Our job tracking tables in puck.db are independently designed but were informed by the same tutorial's approach to structuring career data.
