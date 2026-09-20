export const AdActionType = {
  OpenBrowser: "open_browser",
} as const;

export type AdAction = {
  type: typeof AdActionType.OpenBrowser;
  url: string;
};

export type AdSlot = {
  id: string;
  enabled: boolean;
  placement: "menu";
  target: {
    title: string;
    description: string;
    imageUrl: string;
  };
  content: {
    title: string;
    description: string;
    imageUrl: string;
    details: Array<{
      label: string;
      value: string;
    }>;
    /** 为 null 时弹窗不渲染操作按钮（纯展示卡片） */
    button: {
      label: string;
      action: AdAction;
    } | null;
  };
};

export type AdRuntime = {
  slots: AdSlot[];
};
