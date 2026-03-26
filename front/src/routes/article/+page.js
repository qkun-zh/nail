import { error } from '@sveltejs/kit';
import { requester } from '$lib/api/index.js';

export const load = async () => {
    let requestData = [];
    try {
        const realData = await requester.Get("/front-page-articles").send();
        requestData = Array.isArray(realData) ? realData : [];
    } catch (err) {
        console.error('【请求/解析错误】：', err);
        throw error(500, err.message || '获取首页文章失败');
    }

    return { front_page_articles: requestData };
};