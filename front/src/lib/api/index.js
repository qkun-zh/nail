import { createAlova } from 'alova';
import adapterFetch from 'alova/fetch';
import SvelteHook from 'alova/svelte';


export const requester = createAlova({
    id: 'user',
    requestAdapter: adapterFetch(),
    statesHook: SvelteHook,
    timeout: 10000,
    baseURL: 'https://m1.apifoxmock.com/m1/7529433-7265912-default',
    headers: {
        'Content-Type': 'application/json;charset=UTF-8'
    },
    async responded(response, method) {
        return response.json();
    }
});