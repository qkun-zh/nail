import { page } from '$app/state';
export const idAndVersion = function getIdAndVersionOfArticleFromURL() {
    const parts = page.url.pathname.split('/');
    const validParts = parts.filter(part => part.trim() !== '');
    if (validParts.length < 3) {
        return { id: '', version: '' };
    }
    return {
        id: validParts[1],
        version: validParts[2]
    };
}