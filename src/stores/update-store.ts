import { create } from "zustand";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateState {
  updateAvailable: boolean;
  updateVersion: string | null;
  updateBody: string | null;
  updateDate: string | null;
  currentVersion: string | null;
  isChecking: boolean;
  isDownloading: boolean;
  downloadProgress: number;
  downloadTotal: number;
  downloadedBytes: number;
  error: string | null;
  checkForUpdates: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
  dismissUpdate: () => void;
}

let cachedUpdate: Update | null = null;

export const useUpdateStore = create<UpdateState>((set) => ({
  updateAvailable: false,
  updateVersion: null,
  updateBody: null,
  updateDate: null,
  currentVersion: null,
  isChecking: false,
  isDownloading: false,
  downloadProgress: 0,
  downloadTotal: 0,
  downloadedBytes: 0,
  error: null,

  checkForUpdates: async () => {
    set({ isChecking: true, error: null });
    try {
      const update = await check();
      if (update) {
        cachedUpdate = update;
        set({
          updateAvailable: true,
          updateVersion: update.version,
          updateBody: update.body ?? null,
          updateDate: update.date ?? null,
          currentVersion: update.currentVersion,
        });
      } else {
        cachedUpdate = null;
        set({
          updateAvailable: false,
          updateVersion: null,
          updateBody: null,
          updateDate: null,
        });
      }
    } catch (error) {
      console.error("Failed to check for updates:", error);
      set({ error: String(error) });
    } finally {
      set({ isChecking: false });
    }
  },

  downloadAndInstall: async () => {
    if (!cachedUpdate) return;
    set({ isDownloading: true, downloadProgress: 0, downloadTotal: 0, downloadedBytes: 0, error: null });
    try {
      let totalBytes = 0;
      let received = 0;
      await cachedUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? 0;
          set({ downloadTotal: totalBytes });
        } else if (event.event === "Progress") {
          received += event.data.chunkLength;
          const progress = totalBytes > 0 ? Math.round((received / totalBytes) * 100) : 0;
          set({ downloadedBytes: received, downloadProgress: progress });
        } else if (event.event === "Finished") {
          set({ downloadProgress: 100 });
        }
      });
      await relaunch();
    } catch (error) {
      console.error("Failed to download and install update:", error);
      set({ error: String(error) });
    } finally {
      set({ isDownloading: false });
    }
  },

  dismissUpdate: () => {
    cachedUpdate = null;
    set({
      updateAvailable: false,
      updateVersion: null,
      updateBody: null,
      updateDate: null,
      error: null,
    });
  },
}));
