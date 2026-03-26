<script>
    import { page } from '$app/state';
    let current_path = $derived(page.url.pathname)
</script>

<span>0)</span>
<a href="{current_path}/book">book</a><br>
<span>1)</span>
<a href="{current_path}/callsign">callsign</a><br>
<span>2)</span>
<a href="{current_path}/comment">comment</a><br>
<span>3)</span>
<a href="{current_path}/id">id</a><br>


