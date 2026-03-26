
export async function getWasmFromUrl(wasmUrl) {
    const res = await fetch(wasmUrl);
    const buffer = await res.arrayBuffer();
    return WebAssembly.compile(buffer);
}