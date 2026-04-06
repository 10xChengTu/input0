
import { useEffect } from "react";
import { useLocaleStore } from "../i18n";
import { useThemeStore } from "../stores/theme-store";
import { useUpdateStore } from "../stores/update-store";
import { getVersion } from "@tauri-apps/api/app";
import { useState } from "react";
import logoUrl from "../assets/logo.png";

export type PageId = "home" | "history" | "vocabulary" | "data" | "settings";

interface SidebarProps {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
}

const navIcons: Record<PageId, React.ReactNode> = {
  home: (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-[18px] h-[18px]">
      <path strokeLinecap="round" strokeLinejoin="round" d="m2.25 12 8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25" />
    </svg>
  ),
  history: (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-[18px] h-[18px]">
      <path strokeLinecap="round" strokeLinejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
    </svg>
  ),
  vocabulary: (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-[18px] h-[18px]">
      <path strokeLinecap="round" strokeLinejoin="round" d="M12 6.042A8.967 8.967 0 0 0 6 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 0 1 6 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 0 1 6-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0 0 18 18a8.967 8.967 0 0 0-6 2.292m0-14.25v14.25" />
    </svg>
  ),
  data: (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-[18px] h-[18px]">
      <path strokeLinecap="round" strokeLinejoin="round" d="M7.5 7.5h-.75A2.25 2.25 0 0 0 4.5 9.75v7.5a2.25 2.25 0 0 0 2.25 2.25h7.5a2.25 2.25 0 0 0 2.25-2.25v-7.5a2.25 2.25 0 0 0-2.25-2.25h-.75m-6 3.75 3 3m0 0 3-3m-3 3V1.5m6 9h.75a2.25 2.25 0 0 1 2.25 2.25v7.5a2.25 2.25 0 0 1-2.25 2.25h-7.5a2.25 2.25 0 0 1-2.25-2.25v-7.5a2.25 2.25 0 0 1 2.25-2.25H9" />
    </svg>
  ),
  settings: (
    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-[18px] h-[18px]">
      <path strokeLinecap="round" strokeLinejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z" />
      <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
    </svg>
  ),
};

const pageIds: PageId[] = ["home", "history", "vocabulary", "data", "settings"];

export function Sidebar({ activePage, onNavigate }: SidebarProps) {
  const { t, locale, toggleLocale } = useLocaleStore();
  const { theme, toggleTheme } = useThemeStore();
  const { updateAvailable, checkForUpdates } = useUpdateStore();
  const [appVersion, setAppVersion] = useState("0.1.0");

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => {});
    checkForUpdates();
  }, []);

  const navLabels: Record<PageId, string> = {
    home: t.sidebar.home,
    history: t.sidebar.history,
    vocabulary: t.sidebar.vocabulary,
    data: t.sidebar.data,
    settings: t.sidebar.settings,
  };

  return (
    <div
      className="flex flex-col w-[200px] flex-shrink-0 h-screen bg-[var(--theme-sidebar-bg)]"
      data-tauri-drag-region
    >
      <div className="px-5 pt-10 pb-4" data-tauri-drag-region>
        <div className="flex items-center gap-2 select-none">
          <img src={logoUrl} alt="Input0" className="w-5 h-5 object-contain" />
          <h1 className="text-base font-semibold text-[var(--theme-on-surface)] tracking-tighter">
            Input0
          </h1>
        </div>
      </div>

      <nav className="flex-1 px-3 space-y-0.5">
        {pageIds.map((id) => {
          const isActive = activePage === id;
          return (
            <button
              key={id}
              onClick={() => onNavigate(id)}
              className={`w-full flex items-center gap-2.5 px-2.5 py-2 rounded-md text-[13px] font-medium transition-colors outline-none ${
                isActive
                  ? "bg-[var(--theme-sidebar-item-active)] text-[var(--theme-sidebar-item-active-text)]"
                  : "text-[var(--theme-sidebar-text)] hover:bg-[var(--theme-sidebar-item-hover)]"
              }`}
            >
              <span className={isActive ? "text-[var(--theme-sidebar-item-active-text)]" : "text-[var(--theme-sidebar-text-muted)]"}>
                {navIcons[id]}
              </span>
              {navLabels[id]}
            </button>
          );
        })}
      </nav>

      <div className="px-3 pb-4 flex items-center justify-between">
        <div className="px-2.5 flex items-center gap-1.5">
          <span className="text-[11px] text-[var(--theme-sidebar-text-muted)] select-none">
            v{appVersion}
          </span>
          {updateAvailable && (
            <span className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--theme-primary)] opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-[var(--theme-primary)]"></span>
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <a
            href="https://github.com/10xChengTu/input0"
            target="_blank"
            rel="noopener noreferrer"
            className="p-1.5 rounded-md text-[var(--theme-sidebar-text-muted)] hover:text-[var(--theme-sidebar-item-active-text)] hover:bg-[var(--theme-sidebar-item-hover)] transition-colors"
            title="GitHub"
          >
            <svg viewBox="0 0 24 24" fill="currentColor" className="w-4 h-4">
              <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"/>
            </svg>
          </a>
          <button
            onClick={toggleLocale}
            className="p-1.5 rounded-md text-[var(--theme-sidebar-text-muted)] hover:text-[var(--theme-sidebar-item-active-text)] hover:bg-[var(--theme-sidebar-item-hover)] transition-colors"
            title={locale === "zh" ? "Switch to English" : "切换到中文"}
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4">
              <path strokeLinecap="round" strokeLinejoin="round" d="m10.5 21 5.25-11.25L21 21m-9-3h7.5M3 5.621a48.474 48.474 0 0 1 6-.371m0 0c1.12 0 2.233.038 3.334.114M9 5.25V3m3.334 2.364C11.176 10.658 7.69 15.08 3 17.502m9.334-12.138c.896.061 1.785.147 2.666.257m-4.589 8.495a18.023 18.023 0 0 1-3.827-5.802" />
            </svg>
          </button>
          <button
            onClick={toggleTheme}
            className="p-1.5 rounded-md text-[var(--theme-sidebar-text-muted)] hover:text-[var(--theme-sidebar-item-active-text)] hover:bg-[var(--theme-sidebar-item-hover)] transition-colors"
            title={theme === "dark" ? (locale === "zh" ? "切换到浅色模式" : "Switch to light mode") : (locale === "zh" ? "切换到深色模式" : "Switch to dark mode")}
          >
            {theme === "dark" ? (
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4">
                <path strokeLinecap="round" strokeLinejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" />
              </svg>
            ) : (
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4">
                <path strokeLinecap="round" strokeLinejoin="round" d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" />
              </svg>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
