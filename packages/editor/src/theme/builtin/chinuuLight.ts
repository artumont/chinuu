import { defaultFonts, type ChinuuTheme } from "../tokens.ts";

export const chinuuLight: ChinuuTheme = {
  name: "Chinuu Light",
  dark: false,
  ...defaultFonts,
  background: "#ffffff",
  foreground: "#1f2328",
  caret: "#7c5cff",
  selection: "#d7d4f0",
  activeLine: "rgba(124, 92, 255, 0.055)",
  mark: "#9198a1",
  headingMuted: "#57606a",
  link: "#0969da",
  strikethrough: "#57606a",
  quoteText: "#57606a",
  quoteBorder: "#d0d7de",
  codeBackground: "#f6f8fa",
  codeForeground: "#1f2328",
  inlineCodeBackground: "#f6f8fa",
  inlineCodeBorder: "#e4e8ee",
  tableBackground: "#fafbfc",
  tableForeground: "#444c56",
  rule: "#b9c0c8",
  codeKeyword: "#770088", // #708
  codeString: "#aa1111", // #a11
  codeNumber: "#221199", // #219
  codeComment: "#994400", // #940
  codeFunction: "#0000ff", // #00f
  codeType: "#008855", // #085
  menuBackground: "#ffffff",
  menuBorder: "#d0d7de", // quoteBorder
  menuHighlight: "rgba(124, 92, 255, 0.10)",
};
