export interface Profile {
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
}

export interface Experience {
	id: number;
	company_name: string;
	title: string;
	location: string;
	start_date: string;
	end_date: string | null;
	is_current: boolean;
	summary: string;
	bullet_points: string[];
	display_order: number;
}

export interface Skill {
	id: number;
	skill_name: string;
	category: string;
	years_experience: number;
	last_used: string;
}

export interface Education {
	id: number;
	degree: string;
	institution: string;
	location: string;
	start_year: string;
	end_year: string;
}

export interface Project {
	id: number;
	title: string;
	slug: string;
	summary: string;
	description: string;
	tech_stack: string[];
	url: string;
	sort_order: number;
}

export interface Article {
	id: number;
	title: string;
	slug: string;
	summary: string;
	content: string;
	tags: string[];
	published_at: string | null;
}

export interface Link {
	id: number;
	label: string;
	url: string;
	icon: string;
	sort_order: number;
}

export interface FaqSuggestion {
	id: number;
	question: string;
}

export interface ChatMessage {
	role: 'user' | 'assistant';
	content: string;
}

export interface FitVerdict {
	verdict: 'strong_fit' | 'worth_conversation' | 'probably_not';
	headline: string;
	opening: string;
	gaps: Array<{ requirement: string; gap_title: string; explanation: string }>;
	transfers: Array<{ skill: string; relevance: string }>;
	recommendation: string;
}

export interface Agent {
	id: number;
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
}
