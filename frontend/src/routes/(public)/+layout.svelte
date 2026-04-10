<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import Header from '$lib/components/Header.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import AiPane from '$lib/components/ai/AiPane.svelte';

	let { children } = $props();
	let aiPaneOpen = $state(false);

	let isHome = $derived(page.url.pathname === '/');

	// Listen for toggle-ai-pane custom event from Hero's Ask AI card
	onMount(() => {
		function handleToggleAi() {
			aiPaneOpen = !aiPaneOpen;
		}
		document.addEventListener('toggle-ai-pane', handleToggleAi);
		return () => document.removeEventListener('toggle-ai-pane', handleToggleAi);
	});
</script>

{#if !isHome}
	<Header />
{/if}
<main class:home={isHome}>
	{@render children()}
</main>
<Footer />
<AiPane bind:open={aiPaneOpen} />

<style>
	main {
		min-height: calc(100vh - var(--nav-height) - 8rem);
	}

	main.home {
		/* Home page has no nav bar — hero fills full viewport */
		min-height: 0;
	}
</style>
