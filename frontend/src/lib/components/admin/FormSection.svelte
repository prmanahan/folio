<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    tier = 'public',
    collapsed = false,
    children,
  }: {
    title: string;
    tier?: 'public' | 'ai' | 'private';
    collapsed?: boolean;
    children: Snippet;
  } = $props();

  const badgeLabel: Record<string, string> = {
    public: 'PUBLIC',
    ai: 'AI',
    private: 'PRIVATE',
  };

  const badgeStyle: Record<string, string> = {
    public: 'background: var(--nb-green-dim); color: #7fd4a8;',
    ai: 'background: #2a3030; color: #7dbcbc; border: 1px solid #3a5050;',
    private: 'background: #3a2020; color: #c87070; border: 1px solid #5a3030;',
  };

  const helperText: Record<string, string> = {
    public: 'Visible on your portfolio site',
    ai: 'Used by the AI assistant',
    private: 'Only visible to you',
  };

  const outerBorderStyle = $derived(tier === 'private'
    ? 'background: var(--nb-bg2); border: 1px solid #3a2020; border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;'
    : 'background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;');

  const tierHeadingStyle = 'display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.75rem; padding-bottom: 0.5rem; border-bottom: 1px solid var(--nb-border);';
  const tierLabelStyle = 'font-size: 0.6875rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--nb-text3); font-weight: 600;';
  const badgeBaseStyle = 'padding: 0.0625rem 0.375rem; border-radius: 0.1875rem; font-size: 0.625rem; font-weight: 600; letter-spacing: 0.05em;';
  const helperStyle = 'font-size: 0.6875rem; color: var(--nb-text3); margin-left: auto;';
</script>

{#if tier === 'private'}
  <details open={!collapsed} style={outerBorderStyle}>
    <summary style={tierHeadingStyle + ' cursor: pointer; list-style: none;'}>
      <span style={tierLabelStyle}>{title}</span>
      <span style="{badgeStyle[tier]} {badgeBaseStyle}">{badgeLabel[tier]}</span>
      <span style={helperStyle}>{helperText[tier]}</span>
    </summary>
    <div>
      {@render children()}
    </div>
  </details>
{:else}
  <div style={outerBorderStyle}>
    <div style={tierHeadingStyle}>
      <span style={tierLabelStyle}>{title}</span>
      <span style="{badgeStyle[tier]} {badgeBaseStyle}">{badgeLabel[tier]}</span>
      <span style={helperStyle}>{helperText[tier]}</span>
    </div>
    {@render children()}
  </div>
{/if}
