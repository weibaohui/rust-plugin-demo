// ../reuters_plugin/ui/panel.tsx
function mount(container, deps) {
  const { React, createRoot, pluginId, api } = deps;
  const { useState, useCallback, useEffect, createElement: h } = React;
  function ReutersPanel() {
    const [dateline, setDateline] = useState("LONDON");
    const [status, setStatus] = useState(null);
    useEffect(() => {
      api.getSettings(pluginId).then((s) => {
        if (s?.dateline) setDateline(s.dateline);
      }).catch(() => {
      });
    }, []);
    const handleSave = useCallback(async () => {
      setStatus("saving");
      try {
        await api.saveSettings(pluginId, { dateline });
        setStatus("success");
        setTimeout(() => setStatus(null), 2e3);
      } catch {
        setStatus("error");
        setTimeout(() => setStatus(null), 2e3);
      }
    }, [dateline]);
    return h(
      "div",
      { className: "plugin-panel" },
      h("h3", { className: "panel-title" }, "\u{1F4F0} \u8DEF\u900F\u793E\u63A7\u5236\u9762\u677F"),
      h("p", { className: "panel-desc" }, "\u914D\u7F6E\u8DEF\u900F\u793E\u7684\u7535\u5934\uFF08dateline\uFF09\u504F\u597D\u8BBE\u7F6E"),
      h(
        "div",
        { className: "field" },
        h("label", { className: "field-label" }, "\u7535\u5934 (Dateline)"),
        h("input", {
          className: "field-input",
          value: dateline,
          onChange: (e) => setDateline(e.target.value),
          placeholder: "\u5982 LONDON, NEW YORK\u2026"
        })
      ),
      h(
        "div",
        { className: "field" },
        h("label", { className: "field-label" }, "\u5F53\u524D\u63D2\u4EF6 ID"),
        h("code", { className: "field-code" }, pluginId)
      ),
      h("button", {
        className: "btn btn-primary",
        onClick: handleSave,
        disabled: status === "saving"
      }, status === "saving" ? "\u23F3 \u4FDD\u5B58\u4E2D\u2026" : "\u{1F4BE} \u4FDD\u5B58\u8BBE\u7F6E"),
      status === "success" && h("p", { className: "msg success" }, "\u2705 \u8BBE\u7F6E\u5DF2\u4FDD\u5B58"),
      status === "error" && h("p", { className: "msg error" }, "\u274C \u4FDD\u5B58\u5931\u8D25\uFF0C\u8BF7\u91CD\u8BD5")
    );
  }
  const root = createRoot(container);
  root.render(h(ReutersPanel));
  return () => root.unmount();
}
export {
  mount
};
