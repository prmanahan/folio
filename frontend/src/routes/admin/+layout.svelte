<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { logout } from '$lib/admin-api';
  import '../../admin.css';

  let { children } = $props();

  const isLogin = $derived(page.url.pathname === '/admin/login');

  interface NavLink {
    href: string;
    label: string;
    icon: string;
    exact?: boolean;
  }

  const navLinks: NavLink[] = [
    { href: '/admin',            label: 'Dashboard',  icon: '▦', exact: true },
    { href: '/admin/profile',    label: 'Profile',    icon: '◈' },
    { href: '/admin/experience', label: 'Experience', icon: '◉' },
    { href: '/admin/skills',     label: 'Skills',     icon: '◆' },
    { href: '/admin/education',  label: 'Education',  icon: '◇' },
    { href: '/admin/projects',   label: 'Projects',   icon: '▣' },
    { href: '/admin/articles',   label: 'Articles',   icon: '▤' },
    { href: '/admin/ai',         label: 'AI Config',  icon: '◎' },
    { href: '/admin/links',      label: 'Links',      icon: '◻' },
    { href: '/admin/agents',     label: 'Agents',     icon: '◈' },
  ];

  function isActive(link: NavLink): boolean {
    if (link.exact) return page.url.pathname === link.href;
    return page.url.pathname.startsWith(link.href);
  }

  async function handleLogout() {
    await logout();
    goto('/admin/login');
  }
</script>

{#if isLogin}
  {@render children()}
{:else}
  <div
    class="admin-shell"
    style="display: flex; height: 100dvh; overflow: hidden;"
  >
    <!-- Side Rail -->
    <aside style="
      width: 200px;
      min-width: 200px;
      background: var(--nb-bg2);
      border-right: 1px solid var(--nb-border);
      display: flex;
      flex-direction: column;
      height: 100dvh;
      overflow-y: auto;
    ">
      <!-- Brand header -->
      <div style="
        padding: 1rem 0.875rem 0.75rem;
        border-bottom: 1px solid var(--nb-border);
      ">
        <div style="
          font-size: 0.8rem;
          font-weight: 600;
          color: var(--nb-text);
          letter-spacing: 0.01em;
          line-height: 1.2;
        ">Peter Manahan</div>
        <div style="
          font-size: 0.6rem;
          font-weight: 500;
          color: var(--nb-gold);
          letter-spacing: 0.14em;
          text-transform: uppercase;
          margin-top: 0.2rem;
        ">Portfolio Admin</div>
      </div>

      <!-- Nav section label -->
      <div style="
        padding: 0.75rem 0.875rem 0.25rem;
      ">
        <span style="
          font-size: 0.6rem;
          font-weight: 500;
          color: var(--nb-text3);
          letter-spacing: 0.12em;
          text-transform: uppercase;
        ">Content</span>
      </div>

      <!-- Nav links -->
      <nav style="flex: 1; padding: 0 0.5rem;">
        {#each navLinks as link}
          {@const active = isActive(link)}
          <a
            href={link.href}
            style="
              display: flex;
              align-items: center;
              gap: 0.5rem;
              padding: 0.375rem 0.625rem;
              border-radius: 0.25rem;
              margin-bottom: 0.125rem;
              font-size: 0.8125rem;
              text-decoration: none;
              transition: background 0.12s, color 0.12s;
              background: {active ? 'var(--nb-bg4)' : 'transparent'};
              color: {active ? 'var(--nb-gold)' : 'var(--nb-text2)'};
              border-left: 2px solid {active ? 'var(--nb-gold)' : 'transparent'};
            "
          >
            <span style="font-size: 0.7rem; opacity: 0.7;">{link.icon}</span>
            <span>{link.label}</span>
          </a>
        {/each}
      </nav>

      <!-- Logout footer -->
      <div style="
        padding: 0.75rem 0.5rem;
        border-top: 1px solid var(--nb-border);
      ">
        <button
          onclick={handleLogout}
          style="
            width: 100%;
            padding: 0.375rem 0.625rem;
            background: transparent;
            border: 1px solid var(--nb-border);
            border-radius: 0.25rem;
            color: var(--nb-text3);
            font-size: 0.75rem;
            cursor: pointer;
            transition: border-color 0.12s, color 0.12s;
            font-family: inherit;
          "
          onmouseenter={(e) => {
            (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--nb-text3)';
            (e.currentTarget as HTMLButtonElement).style.color = 'var(--nb-text2)';
          }}
          onmouseleave={(e) => {
            (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--nb-border)';
            (e.currentTarget as HTMLButtonElement).style.color = 'var(--nb-text3)';
          }}
        >
          Logout
        </button>
      </div>
    </aside>

    <!-- Main content area -->
    <main data-theme="ironworks" style="
      flex: 1;
      overflow-y: auto;
      background: var(--nb-bg);
    ">
      {@render children()}
    </main>
  </div>
{/if}
