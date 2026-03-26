<script>
    import EditorAndRenderer from "$lib/component/article/EditorAndRenderer.svelte";
    import localForage from "localforage";
    import {idAndVersion} from "$lib/func/article/getIdAndVersionOfArticleFromURL.js";
    import Loading from "$lib/component/Loading.svelte";
    import Error from "$lib/component/Error.svelte";

    let {id,version} = idAndVersion()
    let articleLocalStore;
    let initDoc = getInitDoc()
    let config = $state({initDoc: initDoc, showEditor: true, showRenderer: true, writable: false});

    async function getInitDoc(){
        articleLocalStore =  localForage.createInstance({name:"article",storeName:id})
        return await articleLocalStore.getItem(version);
    }
</script>

{#await initDoc}
    <Loading></Loading>
{:then value}
    <EditorAndRenderer bind:config={config}></EditorAndRenderer>
{:catch error}
    <Error></Error>
{/await}
