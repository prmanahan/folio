<script lang="ts">
  let {
    message = $bindable(''),
    type = $bindable<'success' | 'error'>('success'),
  }: {
    message: string;
    type?: 'success' | 'error';
  } = $props();

  $effect(() => {
    if (message) {
      const timer = setTimeout(() => { message = ''; }, 3000);
      return () => clearTimeout(timer);
    }
  });
</script>

{#if message}
  <div class="toast toast-top toast-end z-50">
    <div class="alert {type === 'success' ? 'alert-success' : 'alert-error'} text-sm">
      {message}
    </div>
  </div>
{/if}
