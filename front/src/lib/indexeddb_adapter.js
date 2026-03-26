import localForage from 'localforage';

const nailStore = localForage.createInstance({
    name: "nail",
    storeName: "nail",
    description: "database of nail's frontend",
});

export function createIndexedDBAdapter() {
    return {
        get: async (key) => {
            try {
                return await nailStore.getItem(key);
            } catch (error) {
                console.error('IndexedDB get 失败:', error);
                throw new Error('数据获取失败');
            }
        },
        set: async (key, value) => {
            try {
                await nailStore.setItem(key, value);
            } catch (error) {
                console.error('IndexedDB set 失败:', error);
                throw new Error('数据存储失败');
            }
        },
        remove: async (key) => {
            try {
                await nailStore.removeItem(key);
            } catch (error) {
                console.error('IndexedDB remove 失败:', error);
                throw new Error('数据删除失败');
            }
        },
        clear: async () => {
            try {
                await nailStore.clear();
            } catch (error) {
                console.error('IndexedDB clear 失败:', error);
                throw new Error('缓存清空失败');
            }
        }
    };
}