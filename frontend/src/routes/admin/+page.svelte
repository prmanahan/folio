<script lang="ts">
  import { onMount } from 'svelte';
  import { getDashboard } from '$lib/admin-api';
  import type { DashboardCounts } from '$lib/admin-types';

  let counts = $state<DashboardCounts | null>(null);
  let error = $state('');

  const cards = [
    { key: 'experiences', label: 'Experience',       href: '/admin/experience' },
    { key: 'skills',      label: 'Skills',           href: '/admin/skills' },
    { key: 'education',   label: 'Education',        href: '/admin/education' },
    { key: 'projects',    label: 'Projects',         href: '/admin/projects' },
    { key: 'articles',    label: 'Articles',         href: '/admin/articles' },
    { key: 'links',       label: 'Links',            href: '/admin/links' },
    { key: 'faq_responses',    label: 'FAQ Responses',    href: '/admin/ai' },
    { key: 'gaps_weaknesses',  label: 'Gaps & Weaknesses', href: '/admin/ai' },
    { key: 'ai_instructions',  label: 'AI Instructions',   href: '/admin/ai' },
  ] as const;

  const quickActions = [
    { label: '+ New Experience', href: '/admin/experience' },
    { label: '+ New Article',    href: '/admin/articles' },
    { label: '+ New Project',    href: '/admin/projects' },
    { label: '+ New Skill',      href: '/admin/skills' },
  ];

  onMount(async () => {
    try {
      counts = await getDashboard();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load dashboard';
    }
  });
</script>

<div style="padding: 0;">
  <!-- Page header -->
  <div style="padding: 1.5rem 2rem 0; border-bottom: 1px solid var(--nb-border); margin-bottom: 0;">
    <h1 style="
      font-size: 1.125rem;
      font-weight: 500;
      color: var(--nb-text);
      margin: 0 0 0.1875rem;
      letter-spacing: -0.01em;
    ">Dashboard</h1>
    <p style="
      font-size: 0.75rem;
      color: var(--nb-text3);
      margin: 0 0 1rem;
    ">Overview of your portfolio content</p>
  </div>

  <div style="padding: 2rem 2rem 3rem;">
    <!-- Quick actions -->
    <div style="margin-bottom: 2rem;">
      <div style="
        font-size: 0.625rem;
        font-weight: 500;
        color: var(--nb-text3);
        letter-spacing: 0.12em;
        text-transform: uppercase;
        margin-bottom: 0.625rem;
      ">Quick Actions</div>
      <div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">
        {#each quickActions as action, i}
          <a
            href={action.href}
            style="
              padding: 0.375rem 0.875rem;
              background: {i === 0 ? 'var(--nb-gold-dim)' : 'var(--nb-bg3)'};
              border: 1px solid {i === 0 ? 'var(--nb-gold)' : 'var(--nb-border)'};
              border-radius: 0.25rem;
              color: {i === 0 ? 'var(--nb-text)' : 'var(--nb-text2)'};
              font-size: 0.8125rem;
              font-weight: {i === 0 ? '600' : '400'};
              text-decoration: none;
              transition: border-color 0.12s, color 0.12s, background 0.12s;
            "
            onmouseenter={(e) => {
              const el = e.currentTarget as HTMLAnchorElement;
              if (i === 0) {
                el.style.background = 'var(--nb-gold)';
                el.style.color = 'var(--nb-bg)';
              } else {
                el.style.borderColor = 'var(--nb-gold-dim)';
                el.style.color = 'var(--nb-gold)';
              }
            }}
            onmouseleave={(e) => {
              const el = e.currentTarget as HTMLAnchorElement;
              if (i === 0) {
                el.style.background = 'var(--nb-gold-dim)';
                el.style.color = 'var(--nb-text)';
              } else {
                el.style.borderColor = 'var(--nb-border)';
                el.style.color = 'var(--nb-text2)';
              }
            }}
          >{action.label}</a>
        {/each}
      </div>
    </div>

    <!-- Stats section label -->
    <div style="
      font-size: 0.625rem;
      font-weight: 500;
      color: var(--nb-text3);
      letter-spacing: 0.12em;
      text-transform: uppercase;
      margin-bottom: 0.625rem;
    ">Content Counts</div>

    {#if error}
      <div style="
        padding: 0.75rem 1rem;
        background: color-mix(in srgb, var(--nb-red) 12%, transparent);
        border: 1px solid var(--nb-red);
        border-radius: 0.25rem;
        color: var(--nb-red-text);
        font-size: 0.875rem;
        margin-bottom: 1rem;
      ">{error}</div>
    {:else if !counts}
      <div style="color: var(--nb-text3); font-size: 0.875rem; padding: 1rem 0;">Loading…</div>
    {:else}
      <div style="
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
        gap: 0.75rem;
      ">
        {#each cards as card}
          <a
            href={card.href}
            style="
              display: block;
              background: var(--nb-bg2);
              border: 1px solid var(--nb-border);
              border-radius: 0.5rem;
              padding: 1rem 1.125rem;
              text-decoration: none;
              transition: border-color 0.12s, background 0.12s;
            "
            onmouseenter={(e) => {
              const el = e.currentTarget as HTMLAnchorElement;
              el.style.borderColor = 'var(--nb-gold-dim)';
              el.style.background = 'var(--nb-bg3)';
            }}
            onmouseleave={(e) => {
              const el = e.currentTarget as HTMLAnchorElement;
              el.style.borderColor = 'var(--nb-border)';
              el.style.background = 'var(--nb-bg2)';
            }}
          >
            <div style="
              font-size: 0.6rem;
              font-weight: 500;
              color: var(--nb-text3);
              letter-spacing: 0.12em;
              text-transform: uppercase;
              margin-bottom: 0.375rem;
            ">{card.label}</div>
            <div style="
              font-size: 1.75rem;
              font-weight: 400;
              color: var(--nb-gold);
              font-family: 'IBM Plex Mono', monospace;
              line-height: 1;
            ">{counts[card.key]}</div>
            <div style="
              font-size: 0.6875rem;
              color: var(--nb-text3);
              margin-top: 0.375rem;
            ">→ Manage</div>
          </a>
        {/each}
      </div>
    {/if}
  </div>
</div>
