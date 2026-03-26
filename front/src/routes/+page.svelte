<span>welcome to nail</span>
<br>
<a href="/user">users</a>
<span>can read and write </span>
<a href="/article">articles</a>
<span>here</span>


<br><a href="/test">test</a>
<!--&lt;!&ndash; src/routes/+page.svelte &ndash;&gt;-->
<!--<script>-->
<!--    import { alova } from '../api/index.js';-->

<!--    // 手动创建请求 + 解析 Response 拿到真实 JSON 数据-->
<!--    const helloReq = alova.Get('/hello');-->
<!--    helloReq.send().then(async (response) => {-->
<!--        // 关键：解析 Response 的 body 为 JSON（这一步之前少了！）-->
<!--        const realData = await response.json();-->
<!--        console.log('【Alova 真实响应体】：', realData); // 打印解析后的真实数据-->
<!--        console.log('【响应体 JSON 格式】：', JSON.stringify(realData, nothing, 2));-->
<!--    }).catch(err => {-->
<!--        console.error('【请求错误】：', err);-->
<!--    });-->

<!--    // 响应式兜底-->
<!--    const { data, loading, error } = helloReq.useStates();-->
<!--    $: console.log('【响应式 $data 打印】：', $data);-->
<!--</script>-->

<!--<div style="padding: 20px;">-->
<!--    <h3>Alova 真实响应体（控制台）</h3>-->
<!--    {#if $data}-->
<!--        <pre style="background: #f5f5f5; padding: 10px;">{JSON.stringify($data, nothing, 2)}</pre>-->
<!--    {/if}-->
<!--</div>-->