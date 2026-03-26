export function debounce(fn, delay, immediate = false) {
    let timeoutId;
    let result;

    const debounced = function(...args) {
        return new Promise(async (resolve) => {
            const later = async () => {
                timeoutId = null;
                if (!immediate) {
                    result = await fn.apply(this, args);
                    resolve(result);
                }
            };

            const callNow = immediate && !timeoutId;
            clearTimeout(timeoutId);
            timeoutId = setTimeout(later, delay);

            if (callNow) {
                result = await fn.apply(this, args);
                resolve(result);
            }
        });
    };

    debounced.cancel = function() {
        clearTimeout(timeoutId);
        timeoutId = null;
    };

    return debounced;
}