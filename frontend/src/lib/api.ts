import type { Profile, Experience, Skill, Education, Project, Article, Link, FaqSuggestion, FitVerdict, Agent } from './types';

const BASE = '/api';

async function get<T>(path: string): Promise<T> {
	const res = await fetch(`${BASE}${path}`);
	if (!res.ok) {
		throw new Error(`API error: ${res.status} ${res.statusText}`);
	}
	return res.json();
}

export const api = {
	getProfile: () => get<Profile>('/profile'),
	getExperience: () => get<Experience[]>('/experience'),
	getSkills: () => get<Skill[]>('/skills'),
	getEducation: () => get<Education[]>('/education'),
	getProjects: () => get<Project[]>('/projects'),
	getProject: (slug: string) => get<Project>(`/projects/${encodeURIComponent(slug)}`),
	getArticles: () => get<Article[]>('/articles'),
	getArticle: (slug: string) => get<Article>(`/articles/${encodeURIComponent(slug)}`),
	getLinks: () => get<Link[]>('/links'),
	getAgents: () => get<Agent[]>('/agents'),
	getFaqSuggestions: () => get<FaqSuggestion[]>('/faq/suggestions'),
	chat: (message: string) => fetch('/api/chat', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ message }),
	}),
	fitAnalysis: async (jobDescription: string): Promise<FitVerdict> => {
		const res = await fetch('/api/fit', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ job_description: jobDescription }),
		});
		if (res.status === 429) throw new Error('Rate limit reached. Please try again later.');
		if (res.status === 500) throw new Error('AI features are currently unavailable.');
		if (!res.ok) throw new Error('Something went wrong. Please try again.');
		return res.json();
	},
};
