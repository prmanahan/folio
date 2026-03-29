import { goto } from '$app/navigation';
import type {
  LoginResponse, DashboardCounts,
  ProfileFull, ProfileInput,
  ExperienceFull, ExperienceInput,
  SkillFull, SkillInput,
  Education, EducationInput,
  ProjectFull, ProjectInput,
  ArticleFull, ArticleInput,
  Link, LinkInput,
  FaqFull, FaqInput,
  GapWeakness, GapWeaknessInput,
  AiInstruction, AiInstructionInput,
  ValuesCulture, ValuesCultureInput,
  AgentFull, AgentInput,
} from './admin-types';

const BASE = '/api/admin';

// --- Token management ---

export function getToken(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem('admin_token');
}

export function setToken(token: string, expiresAt: string): void {
  localStorage.setItem('admin_token', token);
  localStorage.setItem('admin_token_expires', expiresAt);
}

export function clearToken(): void {
  localStorage.removeItem('admin_token');
  localStorage.removeItem('admin_token_expires');
}

export function isTokenExpired(): boolean {
  const expires = localStorage.getItem('admin_token_expires');
  if (!expires) return true;
  return new Date(expires) <= new Date();
}

// --- Authenticated fetch ---

async function adminFetch<T>(method: string, path: string, body?: unknown): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {};
  if (token) headers['Authorization'] = `Bearer ${token}`;
  if (body !== undefined) headers['Content-Type'] = 'application/json';

  const res = await fetch(`${BASE}${path}`, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (res.status === 401) {
    clearToken();
    goto('/admin/login');
    throw new Error('Unauthorized');
  }

  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `API error: ${res.status}`);
  }

  if (res.status === 204) return undefined as T;
  return res.json();
}

// --- Auth ---

export async function login(password: string): Promise<LoginResponse> {
  const res = await fetch(`${BASE}/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || 'Login failed');
  }
  const data: LoginResponse = await res.json();
  setToken(data.token, data.expires_at);
  return data;
}

export async function logout(): Promise<void> {
  try {
    await adminFetch('POST', '/logout');
  } finally {
    clearToken();
  }
}

// --- Dashboard ---
export const getDashboard = () => adminFetch<DashboardCounts>('GET', '/dashboard');

// --- Profile (singleton) ---
export const getProfile = () => adminFetch<ProfileFull>('GET', '/profile');
export const updateProfile = (input: ProfileInput) => adminFetch<ProfileFull>('PUT', '/profile', input);

// --- Experience ---
export const listExperience = () => adminFetch<ExperienceFull[]>('GET', '/experience');
export const getExperience = (id: number) => adminFetch<ExperienceFull>('GET', `/experience/${id}`);
export const createExperience = (input: ExperienceInput) => adminFetch<ExperienceFull>('POST', '/experience', input);
export const updateExperience = (id: number, input: ExperienceInput) => adminFetch<ExperienceFull>('PUT', `/experience/${id}`, input);
export const deleteExperience = (id: number) => adminFetch<void>('DELETE', `/experience/${id}`);

// --- Skills ---
export const listSkills = () => adminFetch<SkillFull[]>('GET', '/skills');
export const getSkill = (id: number) => adminFetch<SkillFull>('GET', `/skills/${id}`);
export const createSkill = (input: SkillInput) => adminFetch<SkillFull>('POST', '/skills', input);
export const updateSkill = (id: number, input: SkillInput) => adminFetch<SkillFull>('PUT', `/skills/${id}`, input);
export const deleteSkill = (id: number) => adminFetch<void>('DELETE', `/skills/${id}`);

// --- Education ---
export const listEducation = () => adminFetch<Education[]>('GET', '/education');
export const getEducationById = (id: number) => adminFetch<Education>('GET', `/education/${id}`);
export const createEducation = (input: EducationInput) => adminFetch<Education>('POST', '/education', input);
export const updateEducation = (id: number, input: EducationInput) => adminFetch<Education>('PUT', `/education/${id}`, input);
export const deleteEducation = (id: number) => adminFetch<void>('DELETE', `/education/${id}`);

// --- Projects ---
export const listProjects = () => adminFetch<ProjectFull[]>('GET', '/projects');
export const getProject = (id: number) => adminFetch<ProjectFull>('GET', `/projects/${id}`);
export const createProject = (input: ProjectInput) => adminFetch<ProjectFull>('POST', '/projects', input);
export const updateProject = (id: number, input: ProjectInput) => adminFetch<ProjectFull>('PUT', `/projects/${id}`, input);
export const deleteProject = (id: number) => adminFetch<void>('DELETE', `/projects/${id}`);

// --- Articles ---
export const listArticles = () => adminFetch<ArticleFull[]>('GET', '/articles');
export const getArticle = (id: number) => adminFetch<ArticleFull>('GET', `/articles/${id}`);
export const createArticle = (input: ArticleInput) => adminFetch<ArticleFull>('POST', '/articles', input);
export const updateArticle = (id: number, input: ArticleInput) => adminFetch<ArticleFull>('PUT', `/articles/${id}`, input);
export const deleteArticle = (id: number) => adminFetch<void>('DELETE', `/articles/${id}`);

// --- Links ---
export const listLinks = () => adminFetch<Link[]>('GET', '/links');
export const getLinkById = (id: number) => adminFetch<Link>('GET', `/links/${id}`);
export const createLink = (input: LinkInput) => adminFetch<Link>('POST', '/links', input);
export const updateLink = (id: number, input: LinkInput) => adminFetch<Link>('PUT', `/links/${id}`, input);
export const deleteLink = (id: number) => adminFetch<void>('DELETE', `/links/${id}`);

// --- FAQ ---
export const listFaq = () => adminFetch<FaqFull[]>('GET', '/faq');
export const getFaq = (id: number) => adminFetch<FaqFull>('GET', `/faq/${id}`);
export const createFaq = (input: FaqInput) => adminFetch<FaqFull>('POST', '/faq', input);
export const updateFaq = (id: number, input: FaqInput) => adminFetch<FaqFull>('PUT', `/faq/${id}`, input);
export const deleteFaq = (id: number) => adminFetch<void>('DELETE', `/faq/${id}`);

// --- Gaps ---
export const listGaps = () => adminFetch<GapWeakness[]>('GET', '/gaps');
export const getGap = (id: number) => adminFetch<GapWeakness>('GET', `/gaps/${id}`);
export const createGap = (input: GapWeaknessInput) => adminFetch<GapWeakness>('POST', '/gaps', input);
export const updateGap = (id: number, input: GapWeaknessInput) => adminFetch<GapWeakness>('PUT', `/gaps/${id}`, input);
export const deleteGap = (id: number) => adminFetch<void>('DELETE', `/gaps/${id}`);

// --- AI Instructions ---
export const listInstructions = () => adminFetch<AiInstruction[]>('GET', '/instructions');
export const getInstruction = (id: number) => adminFetch<AiInstruction>('GET', `/instructions/${id}`);
export const createInstruction = (input: AiInstructionInput) => adminFetch<AiInstruction>('POST', '/instructions', input);
export const updateInstruction = (id: number, input: AiInstructionInput) => adminFetch<AiInstruction>('PUT', `/instructions/${id}`, input);
export const deleteInstruction = (id: number) => adminFetch<void>('DELETE', `/instructions/${id}`);

// --- Values (singleton) ---
export const getValues = () => adminFetch<ValuesCulture>('GET', '/values');
export const updateValues = (input: ValuesCultureInput) => adminFetch<ValuesCulture>('PUT', '/values', input);

// --- Agents ---
export const listAgents = () => adminFetch<AgentFull[]>('GET', '/agents');
export const getAgent = (id: number) => adminFetch<AgentFull>('GET', `/agents/${id}`);
export const createAgent = (input: AgentInput) => adminFetch<AgentFull>('POST', '/agents', input);
export const updateAgent = (id: number, input: AgentInput) => adminFetch<AgentFull>('PUT', `/agents/${id}`, input);
export const deleteAgent = (id: number) => adminFetch<void>('DELETE', `/agents/${id}`);
