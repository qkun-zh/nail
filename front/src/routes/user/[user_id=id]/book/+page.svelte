<script>
    import { page } from '$app/state';
    let current_path = $derived(page.url.pathname)
</script>

<a href="{current_path}/add">[+]</a>
