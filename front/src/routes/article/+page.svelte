<script>
    import { v7 as uuidV7 } from 'uuid';
    let { data } = $props();
    let front_page_articles = $derived(data.front_page_articles || []);
</script>

<span>&ensp;&ensp;</span>
<a href="/article/add">[+]</a>
<br>


<!--{#each front_page_articles as article, index (article.id)}-->
<!--    <div style="display: flex; align-items: center; margin-bottom: 8px;">-->
<!--        <span style="margin-right: 8px;">{index+1})</span>-->
<!--        <a href="/article/{article.id}" style="flex: 1; margin-right: 8px;">{article.title}</a>-->
<!--        <a href="/article/delete" style="margin-left: auto;">[x]</a>-->
<!--    </div>-->
<!--{/each}-->
{#each front_page_articles as article, index (article.id)}
    <span>{index.toString(16)})</span>
    <a href="/article/{article.id}">{article.title}</a>
    <br>
{/each}


<span>&ensp;&ensp;</span>
<a href="/article/which?">...</a>
