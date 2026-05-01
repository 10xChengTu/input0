export type Locale = "zh" | "en";

export interface Translations {
  // Sidebar
  sidebar: {
    home: string;
    history: string;
    vocabulary: string;
    data: string;
    settings: string;
  };

  // HomePage
  home: {
    welcome: string;
    subtitle: string;
    modelLabel: string;
    modelLoaded: string;
    modelNotLoaded: string;
    ready: string;
    load: string;
    recommendMessage: string;
    switchModel: string;
    hotkey: string;
    language: string;
    usageTitle: string;
    usageStep1Prefix: string;
    usageStep1Suffix: string;
    usageStep2: string;
    usageStep3: string;
    lastResult: string;
    lastResultHint: string;
    editAndSubmit: string;
    submitCorrection: string;
    submitting: string;
    correctionLearned: (count: number) => string;
    correctionNone: string;
    correctionFailed: string;
    correctionNoApiKey: string;
    correctionPartial: (learned: number, total: number) => string;
    dismissResult: string;
  };

  // HistoryPage
  history: {
    title: string;
    noRecords: string;
    noRecordsHint: string;
    noFilteredRecords: string;
    recordCount: (count: number) => string;
    transcribed: string;
    optimized: string;
    copy: string;
    copied: string;
    copyFailed: string;
    clearAll: string;
    todayPrefix: string;
    retentionLast10: string;
    retentionLastHour: string;
    retentionLastDay: string;
  };

  // SettingsPage
  settings: {
    tabGeneral: string;
    tabApi: string;
    tabModels: string;
    tabCustom: string;
    customPromptTitle: string;
    customPromptDescription: string;
    customPromptEnableLabel: string;
    customPromptInsertTagLabel: string;
    customPromptResetToDefault: string;
    customPromptResetConfirm: string;
    customPromptCharsLabel: string;
    customPromptTagDescriptions: {
      clipboard: string;
      vocabulary: string;
      user_tags: string;
      active_app: string;
      language: string;
      history: string;
    };
    apiConfigTitle: string;
    apiKeyLabel: string;
    apiKeyPlaceholder: string;
    apiKeyHint: string;
    apiBaseUrlLabel: string;
    apiBaseUrlPlaceholder: string;
    apiBaseUrlHint: string;
    modelLabel: string;
    modelPlaceholder: string;
    modelHint: string;
    modelDisabledOption: string;
    modelHintDisabled: string;
    modelAddCustom: string;
    modelAddCustomPrompt: string;
    modelAddCustomConfirm: string;
    modelAddCustomCancel: string;
    modelRemoveCustom: string;
    modelRemoveCustomLabel: string;
    modelCustomGroup: string;
    modelPresetGroup: string;
    testConnection: string;
    testConnectionHint: string;
    testing: string;
    test: string;
    voiceSettingsTitle: string;
    languageLabel: string;
    autoDetect: string;
    hotkeyLabel: string;
    hotkeyHint: string;
    hotkeyChange: string;
    hotkeyRecording: string;
    hotkeyChangeFailed: string;
    hotkeyChanged: string;
    hotkeyPresetOptionSpace: string;
    hotkeyPresetFn: string;
    hotkeyPresetCustom: string;
    hotkeyPresetRightOption: string;
    hotkeyPresetLeftOption: string;
    hotkeyPresetRightCommand: string;
    hotkeyPresetLeftCommand: string;
    hotkeyPresetRightControl: string;
    hotkeyPresetLeftControl: string;
    hotkeyPresetRightShift: string;
    hotkeyPresetLeftShift: string;
    hotkeySingleKeyGroup: string;
    hotkeyComboGroup: string;
    hotkeySingleKeyWarningRightOption: string;
    hotkeySingleKeyWarningCommand: string;
    hotkeySingleKeyWarningFn: string;
    hotkeyPermissionBannerTitle: string;
    hotkeyPermissionBannerBody: string;
    hotkeyPermissionBannerAction: string;
    textStructuringLabel: string;
    textStructuringHint: string;
    sttModelTitle: string;
    recommendTitle: string;
    recommendMessage: string;
    switchLabel: string;
    downloading: string;
    download: string;
    useModel: string;
    deleteModel: string;
    active: string;
    downloaded: string;
    notDownloaded: string;
    loadingModels: string;
    noModels: string;
    statusTitle: string;
    loaded: string;
    notLoaded: string;
    saveSettings: string;
    saving: string;
    // Toast messages
    settingsSaved: string;
    settingsSaveFailed: string;
    modelSwitched: string;
    modelSwitchFailed: string;
    modelDownloaded: string;
    modelDownloadFailed: string;
    modelDeleted: string;
    modelDeleteFailed: string;
    recommendSwitched: string;
    recommendSwitchFailed: string;
    userTagsTitle: string;
    userTagsHint: string;
    userTagsGroupIdentity: string;
    userTagsGroupTechStack: string;
    userTagsGroupIndustry: string;
    userTagsGroupUseCase: string;
    tagLabels: { [key: string]: string };
    permissionsTitle: string;
    accessibilityLabel: string;
    accessibilityHint: string;
    accessibilityGranted: string;
    accessibilityNotGranted: string;
    accessibilityGrant: string;
    accessibilityOpenSettings: string;
    microphoneLabel: string;
    microphoneHint: string;
    microphoneGranted: string;
    microphoneNotGranted: string;
    microphoneNotDetermined: string;
    microphoneGrant: string;
    microphoneOpenSettings: string;
    inputDeviceLabel: string;
    inputDeviceHint: string;
    inputDeviceDefault: string;
    hfEndpointLabel: string;
    hfEndpointHint: string;
    hfEndpointPlaceholder: string;
  };

  // VocabularyPage
  vocabulary: {
    title: string;
    subtitle: string;
    addEntry: string;
    termLabel: string;
    termPlaceholder: string;
    add: string;
    adding: string;
    remove: string;
    empty: string;
    emptyHint: string;
    entryCount: (count: number) => string;
    search: string;
    searchPlaceholder: string;
    addSuccess: string;
    addFailed: string;
    removeFailed: string;
    duplicate: string;
    validating: string;
  };

  // DataPage
  data: {
    title: string;
    subtitle: string;
    exportTitle: string;
    exportDescription: string;
    exportButton: string;
    exporting: string;
    exportSuccess: string;
    exportFailed: string;
    exportCancelled: string;
    importTitle: string;
    importDescription: string;
    importButton: string;
    importing: string;
    importSuccess: string;
    importFailed: string;
    importCancelled: string;
    importInvalidFormat: string;
    importConfirmTitle: string;
    importConfirmMessage: string;
    importConfirmYes: string;
    importConfirmNo: string;
    includeHistory: string;
    includeVocabulary: string;
    includeSettings: string;
    historyCount: (count: number) => string;
    vocabularyCount: (count: number) => string;
  };

  // Overlay
  overlay: {
    modelNotLoaded: string;
    modelLoadFailed: string;
    transcriptionFailed: string;
    noMicrophone: string;
    audioError: string;
    networkError: string;
    llmError: string;
    pasteError: string;
    configError: string;
    cancelled: string;
    genericError: string;
    optimizationSkipped: string;
  };

  // Common
  common: {
    loading: string;
  };

  // Update
  update: {
    title: string;
    checkForUpdates: string;
    checking: string;
    available: string;
    availableMessage: (version: string) => string;
    noUpdate: string;
    upToDate: string;
    downloading: string;
    downloadAndInstall: string;
    restartNow: string;
    releaseNotes: string;
    currentVersion: string;
    newVersion: string;
    checkFailed: string;
    downloadFailed: string;
    dismiss: string;
  };

  // Onboarding
  onboarding: {
    title: string;
    subtitle: string;
    stepModel: string;
    stepModelDone: string;
    stepModelHint: string;
    stepApiKey: string;
    stepApiKeyDone: string;
    stepApiKeyHint: string;
    stepUserTags: string;
    stepUserTagsDone: string;
    stepUserTagsHint: string;
    complete: string;
    allDone: string;
  };
}
