<script>
    import { onMount, onDestroy } from 'svelte';
    import { getContext } from "svelte";
    import { proxy, releaseProxy } from "comlink";
    import { VList } from "virtua/svelte";
    import { Text } from "@codemirror/state";

    let { rendererConfig = $bindable({}) } = $props();
    let parserWorkerProxy = null;
    let virtualScrollingData = $state(Text.empty);
    let virtualScrollingDataJSON = $state([]);
    let localVersion = 0;
    let recvProxy = null;
    let isCallbackRegistered = false;
    let isDestroyed = $state(false);

    function createRecvHandler() {
        return function recv(wrappedHtmlDiff) {
            if (isDestroyed) return;

            if(!checkIfVersionLegal(wrappedHtmlDiff.version)){
                console.log("版本不匹配")
                return;
            }
            let diff = wrappedHtmlDiff.data;
            virtualScrollingDataJSON = applyPatch(virtualScrollingDataJSON, diff);
        }
    }

    function applyPatch(oldData, diffs) {
        console.log(diffs);
        let index = 0;
        diffs.forEach(diff => {
            let {count,added,removed,value} = diff;
            if(added===true&&removed===false){
                console.log("add",index);
                oldData.splice(index,0,...value);
                index+=count;
            }else if(added===false&&removed===true){
                oldData.splice(index,count);
                console.log("remove",index);
            }else{
                console.log("save",index,count);
                index+=count;
            }
        });
        console.log(oldData);
        return oldData
    }

    // function applyPatch(oldData, diffs) {
    //     const newData = [...oldData];
    //     let index = 0;
    //     diffs.forEach(diff => {
    //         const { count, added, removed, value } = diff;
    //
    //         if (added && !removed) {
    //             newData.splice(index, 0, ...value);
    //             index += count;
    //         } else if (!added && removed) {
    //             newData.splice(index, count);
    //         } else {
    //             index += count;
    //         }
    //     });
    //     return newData;
    // }

    function checkIfVersionLegal(remoteVersion) {
        if (remoteVersion <= localVersion) {
            return false;
        } else {
            localVersion = remoteVersion;
            return true;
        }
    }

    onMount(async () => {
        let retryCount = 0;

        while (!parserWorkerProxy && retryCount < 8) {
            parserWorkerProxy = getContext(rendererConfig.parserWorkerProxyKey);
            console.log(parserWorkerProxy)
            retryCount++;
            await new Promise(resolve => setTimeout(resolve, 4));
        }

        if (!parserWorkerProxy) {
            console.error("获取parserWorkerProxy失败");
            return;
        }

        const recv = createRecvHandler();
        recvProxy = proxy(recv);

        retryCount = 0;
        while (!isCallbackRegistered && retryCount < 8) {
            try {
                isCallbackRegistered = await parserWorkerProxy.registerCallbackProxyOfRenderer(recvProxy);
            } catch (e) {
                console.warn("注册回调重试失败:", e);
            }
            retryCount++;
            await new Promise(resolve => setTimeout(resolve, 4));
        }

        if (!isCallbackRegistered) {
            console.error("注册renderer回调失败");
            if (recvProxy) {
                recvProxy[releaseProxy]();
                recvProxy = null;
            }
        }
    });

    onDestroy(async () => {
        isDestroyed = true;

        if (recvProxy) {
            recvProxy = null;
        }

        virtualScrollingData = Text.empty;
        virtualScrollingDataJSON = [];
        localVersion = 0;
        parserWorkerProxy = null;
        isCallbackRegistered = false;
    });
</script>



<VList style="height: 100vh; margin: 0; padding: 0;" data={virtualScrollingDataJSON}>
    {#snippet children(item, index)}
        <div class="w">
            <div class="d">{@html item}</div>
            <div class="n">{index+1}</div>
        </div>
    {/snippet}
</VList>


<style>
    .w {
        position: relative;
        margin: 0;
        padding: 0;
        cursor: default;
        min-height: 1px !important;
    }

    .n {
        position: absolute;
        right: 0;
        top: 0;
        z-index: 10;
        white-space: nowrap;
        background-color: rgb(243, 243, 243);
        color: rgb(107, 108, 108);
        margin: 0;
        padding: 0;
        font-size: xx-small;
        opacity: 1;
    }

    .d {
        width: 100%;
        overflow-wrap: break-word;
        word-break: break-word;
        z-index: 1;
        position: relative;
        margin: 0;
        padding-right: min(24px);
    }

    .d img {
        border: 4px solid transparent;
        border-color: #07fd00;
    }

    .w:hover .n {
        background-color: #a8a8a8;
        color: #07fd00;
    }

    .n:hover {
        opacity: 0;
    }
</style>