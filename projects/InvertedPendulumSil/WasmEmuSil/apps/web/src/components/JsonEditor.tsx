// CodeMirror 6 の JSON エディタ (manifest の下書き編集)。
import { useEffect, useRef } from "react";
import { EditorView, basicSetup } from "codemirror";
import { EditorState, Compartment } from "@codemirror/state";
import { json, jsonParseLinter } from "@codemirror/lang-json";
import { linter, lintGutter } from "@codemirror/lint";

interface Props {
  value: string;
  onChange?: (v: string) => void;
  readOnly?: boolean;
  height?: string;
}

export function JsonEditor({ value, onChange, readOnly = false, height = "60vh" }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const ro = useRef(new Compartment());
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (!host.current) return;
    const state = EditorState.create({
      doc: value,
      extensions: [
        basicSetup,
        json(),
        linter(jsonParseLinter()),
        lintGutter(),
        ro.current.of(EditorState.readOnly.of(readOnly)),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onChangeRef.current?.(u.state.doc.toString());
        }),
        EditorView.theme({ "&": { height, fontSize: "13px" }, ".cm-scroller": { overflow: "auto" } }),
      ],
    });
    view.current = new EditorView({ state, parent: host.current });
    return () => {
      view.current?.destroy();
      view.current = null;
    };
    // 初回だけ。value の外部変更は下の effect で反映する
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const v = view.current;
    if (!v) return;
    const cur = v.state.doc.toString();
    if (cur !== value) v.dispatch({ changes: { from: 0, to: cur.length, insert: value } });
  }, [value]);

  useEffect(() => {
    view.current?.dispatch({ effects: ro.current.reconfigure(EditorState.readOnly.of(readOnly)) });
  }, [readOnly]);

  return <div ref={host} className="json-editor" data-testid="json-editor" />;
}
