<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import Header from '$lib/components/Header.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import AiPane from '$lib/components/ai/AiPane.svelte';
	import { api } from '$lib/api';
	import { getInitials } from '$lib/utils';

	let { children } = $props();
	let aiPaneOpen = $state(false);

	// Profile initials for the monogram:
	//   undefined = loading (shows skeleton)
	//   null      = error   (shows "?" fallback)
	//   string    = initials derived from profile name
	let initials: string | null | undefined = $state(undefined);

	// Listen for toggle-ai-pane custom event from Hero's Ask AI card.
	// NOTE: onMount must be synchronous — an async callback returns a Promise,
	// which Svelte cannot use as a cleanup function (the listener would leak).
	// Fire the profile fetch as a detached promise inside the sync body.
	onMount(() => {
		function handleToggleAi() {
			aiPaneOpen = !aiPaneOpen;
		}
		document.addEventListener('toggle-ai-pane', handleToggleAi);

		// Fire-and-forget profile fetch for monogram initials — no cleanup needed here
		api.getProfile()
			.then((profile) => {
				initials = getInitials(profile.name);
			})
			.catch(() => {
				initials = null;
			});

		return () => document.removeEventListener('toggle-ai-pane', handleToggleAi);
	});
</script>

<Header {initials} />
<main>
	{@render children()}
</main>
<Footer />
<AiPane bind:open={aiPaneOpen} />

<style>
	main {
		min-height: calc(100vh - var(--nav-height) - 8rem);
	}
</style>
