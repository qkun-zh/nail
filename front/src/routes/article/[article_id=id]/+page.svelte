<script>
    import { page } from '$app/state';
    let current_path = $derived(page.url.pathname)

    function joinStringsWithSeparator(major, minor, patch, separator) {
        const s1 = String(major);
        const s2 = String(minor);
        const s3 = String(patch);
        const sep = String(separator);
        return [s1, s2, s3].join(sep);
    }

</script>

<span>&ensp;&ensp;</span>
<a href="{current_path}/draft">[+]</a>
<br>

<span>1)</span>
<a href="{current_path}/1.2.1">1.2.1</a>
<br>

<input placeholder="文章名">
<button>修改</button>
<br>




