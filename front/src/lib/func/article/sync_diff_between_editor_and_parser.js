import { ViewPlugin } from "@codemirror/view";
import { getSyncedVersion, receiveUpdates, sendableUpdates } from "@codemirror/collab";
import { ChangeSet } from "@codemirror/state";
import pLimit from 'p-limit';
import { debounce } from "lodash";

export function syncDiff(parserWorkerProxy) {
    return ViewPlugin.fromClass(class {
        constructor(view) {

            this.view = view;
            this.limit = pLimit(1);
            this.lastVersion = 0;

            this.cacheRelatedThing = {
                capNum: 1024,
                capLen: 65536,
                nowNum: 0,
                nowLen: 0,
                trigger: false,
            };


            this.parserWorkerProxy = parserWorkerProxy;

            this.debouncedSendDiff = debounce(() => {
                this.sendDiff().catch(e => console.error(e));
            }, 512, { maxWait: 8192 });

            const selfRef = new WeakRef(this);
            this.cacheRelatedThingProxy = new Proxy(this.cacheRelatedThing, {
                set(target, prop, value, receiver) {
                    const isSuccess = Reflect.set(target, prop, value, receiver);
                    if (prop === "trigger") {
                        const self = selfRef.deref();
                        if (!self) return isSuccess;
                        if (self.checkIfNowOverCap()) {
                            self.debouncedSendDiff.cancel();
                            self.sendDiff().catch(e => console.error(e));
                        } else {
                            self.debouncedSendDiff();
                        }
                    }
                    return isSuccess;
                }
            });
        }

        update(update) {
            if (!update.docChanged || update.startState.doc.eq(update.state.doc)) return;
            this.renewOldNowToNewNow(update);
        }

        destroy() {
            if (this.debouncedSendDiff) {
                this.debouncedSendDiff.cancel();
                this.debouncedSendDiff = null;
            }

            if (this.limit) {
                this.limit.clearQueue();
                this.limit = null;
            }

            if (this.cacheRelatedThing) {
                this.cacheRelatedThing = null;
            }
            this.cacheRelatedThingProxy = null;

            this.view = null;
            this.lastVersion = null;
        }

        clearOldNowAndSetNewNow() {
            this.cacheRelatedThing.nowNum = 0;
            this.cacheRelatedThing.nowLen = 0;
        }

        renewOldNowToNewNow(update) {
            this.cacheRelatedThing.nowNum += 1;
            update.changes.iterChanges((fromA, toA, _, __, inserted) => {
                this.cacheRelatedThing.nowLen += Math.max(toA - fromA, inserted.length) + inserted.lines;
            });
            this.cacheRelatedThingProxy.trigger = !this.cacheRelatedThingProxy.trigger;
        }

        checkIfNowOverCap() {
            return (this.cacheRelatedThingProxy.nowNum >= this.cacheRelatedThingProxy.capNum) ||
                (this.cacheRelatedThingProxy.nowLen >= this.cacheRelatedThingProxy.capLen);
        }

        compress(updates, state) {
            if (!updates || updates.length === 0) return { changes: [] };

            const diffArray = [];
            const len = updates.length;
            let finalChangeSet = ChangeSet.empty();

            for (let i = 0; i < len; i++) {
                const update = updates[i];
                if (update) {
                    finalChangeSet = finalChangeSet.compose(update.changes);
                    updates[i] = null;
                }
            }

            finalChangeSet.iterChanges((fromA, toA, _, __, inserted) => {
                diffArray.push({
                    from: fromA,
                    to: toA,
                    insert: inserted.toJSON()
                });
            });

            finalChangeSet = null;
            updates.length = 0;
            updates = null;

            return {changes: diffArray};
        }

        async sendDiff() {
            try {
                await this.limit(async () => {
                    let diffRaw = null;
                    try {
                        diffRaw = sendableUpdates(this.view.state);
                        this.view.dispatch(receiveUpdates(this.view.state, diffRaw));
                        const nowVersion = getSyncedVersion(this.view.state);
                        if (this.lastVersion === nowVersion) { return; }
                        this.lastVersion = nowVersion;
                        this.clearOldNowAndSetNewNow();
                        let compressedDiff = this.compress(diffRaw, this.view.state);
                        if (compressedDiff.changes.length === 0) return;
                        await this.parserWorkerProxy.recv({
                            data: compressedDiff,
                            version: nowVersion,
                        });
                    } catch (e) {
                        console.error(e);
                        this.clearOldNowAndSetNewNow();
                    } finally {
                        diffRaw = null;
                    }

                });
            } catch (e) {
                console.error(e);
            }
        }
    });
}