function fallbackEditor() {
  const handles = [];
  return {
    _handles: handles,
    create(parent, value, onChange) {
      const textarea = document.createElement("textarea");
      textarea.className = "homotopy-editor homotopy-editor--fallback";
      textarea.spellcheck = false;
      textarea.value = value;
      textarea.addEventListener("input", () => onChange(textarea.value));
      textarea.addEventListener("keydown", (event) => event.stopPropagation());
      textarea.addEventListener("keyup", (event) => event.stopPropagation());
      parent.replaceChildren(textarea);
      const handle = { textarea, parent, onChange, diagnostics: [] };
      handles.push(handle);
      return handle;
    },
    setValue(editor, value) {
      if (editor.textarea && editor.textarea.value !== value) {
        editor.textarea.value = value;
      }
    },
    setDiagnostics() {},
    destroy(editor) {
      const index = handles.indexOf(editor);
      if (index >= 0) handles.splice(index, 1);
      editor.textarea?.remove();
      editor.destroyed = true;
    },
  };
}

window.HomotopyEditor = fallbackEditor();

Promise.all([
  import("https://esm.sh/@codemirror/state@6.4.1"),
  import("https://esm.sh/@codemirror/view@6.26.3?deps=@codemirror/state@6.4.1"),
  import("https://esm.sh/@codemirror/commands@6.6.0?deps=@codemirror/state@6.4.1,@codemirror/view@6.26.3"),
  import("https://esm.sh/@codemirror/language@6.10.2?deps=@codemirror/state@6.4.1,@codemirror/view@6.26.3,@lezer/highlight@1.2.0"),
  import("https://esm.sh/@lezer/highlight@1.2.0"),
])
  .then(([state, view, commands, language, highlight]) => {
    const { EditorState, StateEffect, StateField } = state;
    const {
      Decoration,
      EditorView,
      keymap,
      lineNumbers,
      highlightActiveLine,
      highlightActiveLineGutter,
    } = view;
    const { defaultKeymap, history, historyKeymap } = commands;
    const {
      HighlightStyle,
      StreamLanguage,
      syntaxHighlighting,
      defaultHighlightStyle,
    } = language;
    const { tags } = highlight;

    const homotopyLanguage = StreamLanguage.define({
      name: "homotopy",
      token(stream) {
        if (stream.eatSpace()) return null;
        if (stream.match("//")) {
          stream.skipToEnd();
          return "comment";
        }
        if (stream.match(/"(?:[^"\\]|\\.)*"/)) return "string";
        if (stream.match(/\b(?:cell|prove|schema|macro|use|as|show|title|author|abstract|id|inv)\b/)) {
          return "keyword";
        }
        if (stream.match(/[0-9]+/)) return "number";
        if (stream.match(/[A-Z][A-Za-z0-9_.]*/)) return "typeName";
        if (stream.match(/[a-z_][A-Za-z0-9_.]*/)) return "variableName";
        if (stream.match(/<->|->|[{}():;,.*<>]/)) return "operator";
        stream.next();
        return null;
      },
    });

    const homotopyHighlightStyle = HighlightStyle.define([
      { tag: tags.keyword, color: "#0f766e", fontWeight: "700" },
      { tag: tags.string, color: "#9f1239" },
      { tag: tags.number, color: "#7c3aed" },
      { tag: tags.comment, color: "#64748b", fontStyle: "italic" },
      { tag: tags.typeName, color: "#1d4ed8", fontWeight: "600" },
      { tag: tags.variableName, color: "#334155" },
      { tag: tags.operator, color: "#475569" },
    ]);

    const setDiagnosticsEffect = StateEffect.define();

    function normalizeSeverity(diagnostic) {
      return diagnostic?.severity === "Warning" ? "warning" : "error";
    }

    function spanFor(doc, diagnostic) {
      if (doc.length === 0) return { start: 0, end: 0 };
      const rawStart = Number(diagnostic?.span?.start ?? 0);
      const rawEnd = Number(diagnostic?.span?.end ?? rawStart + 1);
      const start = Math.max(0, Math.min(doc.length, rawStart));
      const end = Math.max(start, Math.min(doc.length, Math.max(rawEnd, start + 1)));
      return { start, end };
    }

    function buildDiagnosticDecorations(doc, diagnostics) {
      const ranges = [];
      for (const diagnostic of diagnostics || []) {
        const severity = normalizeSeverity(diagnostic);
        const { start, end } = spanFor(doc, diagnostic);
        const message = diagnostic?.message || "Diagnostic";
        const line = doc.lineAt(start);
        ranges.push(
          Decoration.line({
            attributes: {
              class: `homotopy-diagnostic-line homotopy-diagnostic-line--${severity}`,
              title: message,
            },
          }).range(line.from),
        );
        ranges.push(
          Decoration.mark({
            class: `homotopy-diagnostic homotopy-diagnostic--${severity}`,
            attributes: { title: message },
          }).range(start, end),
        );
      }
      return Decoration.set(ranges, true);
    }

    const diagnosticField = StateField.define({
      create() {
        return Decoration.none;
      },
      update(decorations, transaction) {
        let next = decorations.map(transaction.changes);
        for (const effect of transaction.effects) {
          if (effect.is(setDiagnosticsEffect)) {
            next = buildDiagnosticDecorations(transaction.state.doc, effect.value);
          }
        }
        return next;
      },
      provide(field) {
        return EditorView.decorations.from(field);
      },
    });

    const theme = EditorView.theme({
      "&": {
        height: "100%",
        fontSize: "13px",
        color: "#1f2937",
        backgroundColor: "#ffffff",
      },
      ".cm-scroller": {
        fontFamily: "\"IBM Plex Mono\", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
        lineHeight: "1.55",
      },
      ".cm-gutters": {
        backgroundColor: "#f8fafc",
        color: "#94a3b8",
        borderRight: "1px solid #e5e7eb",
      },
      ".cm-activeLine": { backgroundColor: "#f0fdfa" },
      ".cm-activeLineGutter": { backgroundColor: "#ccfbf1" },
      ".cm-content": { padding: "12px 0" },
      ".cm-line": { padding: "0 14px" },
    });

    const previousBridge = window.HomotopyEditor;
    const bridge = {
      create(parent, value, onChange) {
        const editor = new EditorView({
          state: EditorState.create({
            doc: value,
            extensions: [
              lineNumbers(),
              highlightActiveLine(),
              highlightActiveLineGutter(),
              history(),
              keymap.of([...defaultKeymap, ...historyKeymap]),
              homotopyLanguage,
              diagnosticField,
              syntaxHighlighting(defaultHighlightStyle),
              syntaxHighlighting(homotopyHighlightStyle),
              theme,
              EditorView.lineWrapping,
              EditorView.domEventHandlers({
                keydown(event) {
                  event.stopPropagation();
                  return false;
                },
                keyup(event) {
                  event.stopPropagation();
                  return false;
                },
              }),
              EditorView.updateListener.of((update) => {
                if (update.docChanged) {
                  onChange(update.state.doc.toString());
                }
              }),
            ],
          }),
        });
        parent.replaceChildren(editor.dom);
        return { editor, diagnostics: [] };
      },
      setValue(handle, value) {
        if (!handle.editor) return;
        const current = handle.editor.state.doc.toString();
        if (current === value) return;
        handle.editor.dispatch({
          changes: { from: 0, to: current.length, insert: value },
        });
      },
      setDiagnostics(handle, diagnostics) {
        handle.diagnostics = diagnostics || [];
        handle.editor?.dispatch({
          effects: setDiagnosticsEffect.of(handle.diagnostics),
        });
      },
      destroy(handle) {
        handle.editor?.destroy();
        handle.destroyed = true;
      },
    };
    window.HomotopyEditor = bridge;

    for (const handle of previousBridge._handles || []) {
      if (handle.destroyed) continue;
      const value = handle.textarea?.value ?? "";
      try {
        Object.assign(handle, bridge.create(handle.parent, value, handle.onChange));
      } catch (error) {
        console.warn("CodeMirror editor upgrade failed; keeping fallback editor.", error);
      }
    }
  })
  .catch((error) => {
    console.warn("CodeMirror modules could not be loaded; using fallback source editor.", error);
  });
