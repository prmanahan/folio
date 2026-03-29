<script lang="ts">
  import { goto } from '$app/navigation';
  import { login } from '$lib/admin-api';

  let password = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleLogin(e: Event) {
    e.preventDefault();
    error = '';
    loading = true;
    try {
      await login(password);
      goto('/admin');
    } catch (err) {
      error = err instanceof Error ? err.message : 'Login failed';
    } finally {
      loading = false;
    }
  }
</script>

<div
  class="admin-shell"
  style="min-height: 100dvh; display: flex; align-items: center; justify-content: center;"
>
  <div style="
    width: 100%;
    max-width: 22rem;
    background: var(--nb-bg2);
    border: 1px solid var(--nb-border);
    border-radius: 0.375rem;
    padding: 2.5rem 2rem;
  ">
    <!-- Site identity -->
    <div style="text-align: center; margin-bottom: 2rem;">
      <div style="
        font-size: 1.4rem;
        font-weight: 600;
        color: var(--nb-text);
        letter-spacing: 0.01em;
        line-height: 1.2;
      ">Peter Manahan</div>
      <div style="
        font-size: 0.625rem;
        font-weight: 500;
        color: var(--nb-gold);
        letter-spacing: 0.2em;
        text-transform: uppercase;
        margin-top: 0.4rem;
      ">Portfolio Admin</div>
      <div style="
        width: 2rem;
        height: 1px;
        background: var(--nb-gold-dim);
        margin: 0.875rem auto 0;
        opacity: 0.6;
      "></div>
    </div>

    <form onsubmit={handleLogin}>
      <div style="margin-bottom: 0.75rem;">
        <input
          type="password"
          placeholder="Password"
          bind:value={password}
          required
          style="
            width: 100%;
            box-sizing: border-box;
            padding: 0.625rem 0.75rem;
            background: var(--nb-bg3);
            border: 1px solid var(--nb-border);
            border-radius: 0.25rem;
            color: var(--nb-text);
            font-size: 0.875rem;
            font-family: inherit;
            outline: none;
            transition: border-color 0.12s;
          "
          onfocus={(e) => (e.currentTarget as HTMLInputElement).style.borderColor = 'var(--nb-gold-dim)'}
          onblur={(e) => (e.currentTarget as HTMLInputElement).style.borderColor = 'var(--nb-border)'}
        />
      </div>

      {#if error}
        <div style="
          padding: 0.5rem 0.75rem;
          background: color-mix(in srgb, var(--nb-red) 15%, transparent);
          border: 1px solid var(--nb-red);
          border-radius: 0.25rem;
          color: var(--nb-red-text);
          font-size: 0.8125rem;
          margin-bottom: 0.75rem;
        ">{error}</div>
      {/if}

      <button
        type="submit"
        disabled={loading}
        style="
          width: 100%;
          padding: 0.625rem 1rem;
          background: var(--nb-gold-dim);
          border: 1px solid var(--nb-gold);
          border-radius: 0.25rem;
          color: var(--nb-text);
          font-size: 0.875rem;
          font-weight: 500;
          font-family: inherit;
          cursor: {loading ? 'not-allowed' : 'pointer'};
          opacity: {loading ? '0.6' : '1'};
          transition: background 0.12s;
          letter-spacing: 0.03em;
        "
        onmouseenter={(e) => { if (!loading) (e.currentTarget as HTMLButtonElement).style.background = 'var(--nb-gold)'; }}
        onmouseleave={(e) => (e.currentTarget as HTMLButtonElement).style.background = 'var(--nb-gold-dim)'}
      >
        {loading ? 'Signing in…' : 'Sign in'}
      </button>
    </form>
  </div>
</div>
