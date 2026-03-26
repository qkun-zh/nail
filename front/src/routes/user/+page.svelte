<script>
    let { data } = $props();
    let front_page_users = $derived(data.front_page_users || []);
</script>


<!--<span>&ensp;&ensp;</span>-->
<!--<span>member</span><span>(which</span>-->
<!--<a href="/user/self">you</a>-->
<!--<span>now are)</span>-->
<!--<span>&lt;-</span>-->
<!--<a href="/user/login">[login]</a>-->
<!--<span>&lt;-</span>-->
<!--<a href="/user/register">[register]</a>-->
<!--<span>&lt;-</span>-->
<!--<span>guest</span>-->
<!--<br>-->
<span>&ensp;&ensp;&ensp;you are now</span>
<br>
<span>&ensp;&ensp;</span>
<span>only a reader -&gt;</span>
<a href="/user/register">[register]</a>
<span>-&gt;</span>
<a href="/user/login">[login]</a>
<span>-&gt; also a writer</span><br>


{#each front_page_users as user, index (user.id)}
    <span>{index.toString(16)})</span>
    <a href="/user/{user.id}">{user.callsign}</a>
    <br>
{/each}


<span>&ensp;&ensp;</span>
<a href="/user/who?">...</a>
