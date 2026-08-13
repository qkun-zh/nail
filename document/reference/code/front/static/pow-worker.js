// PoW Web Worker：把 VDF 证明计算从 JS 主线程搬走——UI 不再被 MinRootVdf::eval
// 的 8k 迭代冻结（HIGH#2-5 / MEDIUM#22 / LOW#16）。
//
// trunk 会把带 hash 的主 bundle（nail_front-<hash>.js）作为 module 引到主页面，
// 但编译期无法知道 hash 文件名，所以本文件是纯静态壳：glue 与 wasm 的地址由
// 主线程（front/src/pow.rs）运行时从 document.scripts 解析后经 message 传入，
// worker 动态 import 同一份 wasm-bindgen glue，再 init 实例化（与主页面同一份
// 产物，只是独立实例与独立内存），最后调用 glue 导出的 pow_prove。
//
// 协议（与 front/src/pow.rs 对应）：
//   主线程 → worker:  {kind:"init", glueUrl, wasmUrl}
//   worker  → 主线程: {kind:"ready"}
//   主线程 → worker:  {kind:"prove", id, challengeId, difficulty, payload}
//   worker  → 主线程: {kind:"solution", id, solution} 或 {kind:"error", id, message}

let glue = null;

self.onmessage = async (ev) => {
  const msg = ev.data || {};
  if (msg.kind === 'init') {
    try {
      const mod = await import(msg.glueUrl);
      // 与主页面生成的 index.html 一致：init 显式传入 wasm 地址。不带参时 glue
      // 会按默认名 nail_front_bg.wasm 派生，与实际 hash 名（nail_front-<hash>_bg.wasm）
      // 不符，所以必须显式传。
      await mod.default({ module_or_path: msg.wasmUrl });
      glue = mod;
      self.postMessage({ kind: 'ready' });
    } catch (e) {
      const message = e && e.message ? String(e.message) : String(e);
      self.postMessage({ kind: 'error', message });
    }
    return;
  }
  if (msg.kind === 'prove') {
    if (!glue || typeof glue.pow_prove !== 'function') {
      self.postMessage({ kind: 'error', id: msg.id, message: 'pow worker not initialized' });
      return;
    }
    try {
      const solution = glue.pow_prove(msg.challengeId, msg.difficulty, msg.payload);
      self.postMessage({ kind: 'solution', id: msg.id, solution });
    } catch (e) {
      const message = e && e.message ? String(e.message) : String(e);
      self.postMessage({ kind: 'error', id: msg.id, message });
    }
    return;
  }
};
