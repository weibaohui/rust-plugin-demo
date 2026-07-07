// ../afp_plugin/ui/panel.tsx
function mount(container, deps) {
  const { React, createRoot, pluginId, api } = deps;
  const { useState, useCallback, useEffect, createElement: h } = React;
  function AfpPanel() {
    const [language, setLanguage] = useState("fr");
    const [status, setStatus] = useState(null);
    useEffect(() => {
      api.getSettings(pluginId).then((s) => {
        if (s?.language) setLanguage(s.language);
      }).catch(() => {
      });
    }, []);
    const handleSave = useCallback(async () => {
      setStatus("saving");
      try {
        await api.saveSettings(pluginId, { language });
        setStatus("success");
        setTimeout(() => setStatus(null), 2e3);
      } catch {
        setStatus("error");
        setTimeout(() => setStatus(null), 2e3);
      }
    }, [language]);
    return h(
      "div",
      { className: "plugin-panel" },
      h("h3", { className: "panel-title" }, "\u{1F4E1} \u6CD5\u65B0\u793E\u63A7\u5236\u9762\u677F"),
      h("p", { className: "panel-desc" }, "\u914D\u7F6E AFP \u7684\u9ED8\u8BA4\u8BED\u8A00\u504F\u597D"),
      h(
        "div",
        { className: "field" },
        h("label", { className: "field-label" }, "\u8BED\u8A00 (Language)"),
        h(
          "select",
          {
            className: "field-input",
            value: language,
            onChange: (e) => setLanguage(e.target.value)
          },
          h("option", { value: "fr" }, "\u{1F1EB}\u{1F1F7} Fran\xE7ais"),
          h("option", { value: "en" }, "\u{1F1EC}\u{1F1E7} English"),
          h("option", { value: "ar" }, "\u{1F1F8}\u{1F1E6} \u0627\u0644\u0639\u0631\u0628\u064A\u0629"),
          h("option", { value: "es" }, "\u{1F1EA}\u{1F1F8} Espa\xF1ol")
        )
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
      status === "success" && h("p", { className: "msg success" }, "\u2705 \u8BED\u8A00\u504F\u597D\u5DF2\u4FDD\u5B58"),
      status === "error" && h("p", { className: "msg error" }, "\u274C \u4FDD\u5B58\u5931\u8D25\uFF0C\u8BF7\u91CD\u8BD5")
    );
  }
  const root = createRoot(container);
  root.render(h(AfpPanel));
  return () => root.unmount();
}
export {
  mount
};
