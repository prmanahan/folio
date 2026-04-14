// Auth
export interface LoginResponse {
  token: string;
  expires_at: string;
}

// Dashboard
export interface DashboardCounts {
  experiences: number;
  skills: number;
  education: number;
  projects: number;
  articles: number;
  links: number;
  faq_responses: number;
  gaps_weaknesses: number;
  ai_instructions: number;
}

// Profile
export interface ProfileFull {
  created_at: string;
  updated_at: string;
  name: string;
  email: string;
  title: string;
  location: string;
  phone: string;
  linkedin_url: string;
  github_url: string;
  twitter_url: string;
  pitch_short: string;
  pitch_long: string;
  availability_status: string;
  availability_date: string;
  remote_preference: string;
  target_titles: string[];
  target_company_stages: string[];
  career_narrative: string;
  looking_for: string;
  not_looking_for: string;
  management_style: string;
  work_style: string;
  salary_min: number | null;
  salary_max: number | null;
}

export interface ProfileInput {
  name: string;
  email: string;
  title: string;
  location: string;
  phone: string;
  linkedin_url: string;
  github_url: string;
  twitter_url: string;
  pitch_short: string;
  pitch_long: string;
  availability_status: string;
  availability_date: string;
  remote_preference: string;
  target_titles: string[];
  target_company_stages: string[];
  career_narrative: string;
  looking_for: string;
  not_looking_for: string;
  management_style: string;
  work_style: string;
  salary_min: number | null;
  salary_max: number | null;
}

// Experience
export interface ExperienceFull {
  id: number;
  created_at: string;
  company_name: string;
  title: string;
  location: string;
  start_date: string;
  end_date: string | null;
  is_current: boolean;
  summary: string;
  bullet_points: string[];
  display_order: number;
  title_progression: string;
  quantified_impact: string[];
  why_joined: string;
  why_left: string;
  actual_contributions: string;
  proudest_achievement: string;
  would_do_differently: string;
  challenges_faced: string;
  lessons_learned: string;
  manager_would_say: string;
  reports_would_say: string;
}

export interface ExperienceInput {
  company_name: string;
  title: string;
  location: string;
  start_date: string;
  end_date: string | null;
  is_current: boolean;
  summary: string;
  bullet_points: string[];
  display_order: number;
  title_progression: string;
  quantified_impact: string[];
  why_joined: string;
  why_left: string;
  actual_contributions: string;
  proudest_achievement: string;
  would_do_differently: string;
  challenges_faced: string;
  lessons_learned: string;
  manager_would_say: string;
  reports_would_say: string;
}

// Skills
export interface SkillFull {
  id: number;
  created_at: string;
  skill_name: string;
  category: string;
  years_experience: number;
  last_used: string;
  self_rating: number;
  evidence: string;
  honest_notes: string;
}

export interface SkillInput {
  skill_name: string;
  category: string;
  years_experience: number;
  last_used: string;
  self_rating: number;
  evidence: string;
  honest_notes: string;
}

// Education
export interface Education {
  id: number;
  degree: string;
  institution: string;
  location: string;
  start_year: string;
  end_year: string;
}

export interface EducationInput {
  degree: string;
  institution: string;
  location: string;
  start_year: string;
  end_year: string;
}

// Projects
export interface ProjectFull {
  id: number;
  title: string;
  slug: string;
  summary: string;
  description: string;
  tech_stack: string[];
  url: string;
  sort_order: number;
  published: boolean;
}

export interface ProjectInput {
  title: string;
  slug: string | null;
  summary: string;
  description: string;
  tech_stack: string[];
  url: string;
  sort_order: number;
  published: boolean;
}

// Articles
export interface ArticleFull {
  id: number;
  title: string;
  slug: string;
  summary: string;
  content: string;
  tags: string[];
  published_at: string | null;
  published: boolean;
}

export interface ArticleInput {
  title: string;
  slug: string | null;
  summary: string;
  content: string;
  tags: string[];
  published_at: string | null;
  published: boolean;
}

// Links
export interface Link {
  id: number;
  label: string;
  url: string;
  icon: string;
  sort_order: number;
}

export interface LinkInput {
  label: string;
  url: string;
  icon: string;
  sort_order: number;
}

// FAQ
export interface FaqFull {
  id: number;
  created_at: string;
  question: string;
  answer: string;
  is_common_question: boolean;
}

export interface FaqInput {
  question: string;
  answer: string;
  is_common_question: boolean;
}

// Gaps
export interface GapWeakness {
  id: number;
  created_at: string;
  gap_type: string;
  description: string;
  why_its_a_gap: string;
  interest_in_learning: boolean;
}

export interface GapWeaknessInput {
  gap_type: string;
  description: string;
  why_its_a_gap: string;
  interest_in_learning: boolean;
}

// AI Instructions
export interface AiInstruction {
  id: number;
  created_at: string;
  instruction_type: string;
  instruction: string;
  priority: number;
}

export interface AiInstructionInput {
  instruction_type: string;
  instruction: string;
  priority: number;
}

// Agents
export interface AgentFull {
  id: number;
  created_at: number;
  name: string;
  role: string;
  short_role: string;
  model: string;
  personality_blurb: string;
  responsibilities: string[];
  avatar_filename: string;
  display_order: number;
  is_featured: boolean;
  is_review_gate: boolean;
  published: boolean;
}

export interface AgentInput {
  name: string;
  role: string;
  short_role: string;
  model: string;
  personality_blurb: string;
  responsibilities: string[];
  avatar_filename: string;
  display_order: number;
  is_featured: boolean;
  is_review_gate: boolean;
  published: boolean;
}

// Values & Culture
export interface ValuesCulture {
  id: number;
  created_at: string;
  must_haves: string;
  dealbreakers: string;
  management_style_preferences: string;
  team_size_preferences: string;
  how_handle_conflict: string;
  how_handle_ambiguity: string;
  how_handle_failure: string;
}

export interface ValuesCultureInput {
  must_haves: string;
  dealbreakers: string;
  management_style_preferences: string;
  team_size_preferences: string;
  how_handle_conflict: string;
  how_handle_ambiguity: string;
  how_handle_failure: string;
}
