import { EditorView, basicSetup } from "codemirror";
import { vim } from "@replit/codemirror-vim";

export const initEditor = (parent: HTMLElement, doc = "") => {
  if (!parent) throw new Error("@chinuu/editor: missing parent element");
  return new EditorView({
    doc,
    extensions: [basicSetup, vim()],
    parent,
  });
};
