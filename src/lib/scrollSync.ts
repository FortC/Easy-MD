// 分屏滚动同步互斥锁：一侧程序化滚动会触发对侧 scroll 事件，
// 锁窗口内的对侧事件被忽略，防止 A→B→A 无限循环。

let lockUntil = 0;

export function lockScrollSync(ms = 160): void {
  lockUntil = performance.now() + ms;
}

export function isScrollSyncLocked(): boolean {
  return performance.now() < lockUntil;
}

export function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}
