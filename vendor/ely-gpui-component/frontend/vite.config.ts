import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";
import { render } from "./src/docs/guides";

const GUIDES = "\0virtual:guides";

/** The guides, rendered at build; a bad one fails the build. */
function docs(): Plugin {
  let files: string[] = [];
  return {
    name: "ely-docs",
    resolveId: (id) => (id === "virtual:guides" ? GUIDES : undefined),
    load(id) {
      if (id !== GUIDES) return;
      const out = render();
      files = out.files;
      files.forEach((file) => this.addWatchFile(file));
      return `export const guides = ${JSON.stringify(out.guides)};`;
    },
    handleHotUpdate({ file, server }) {
      if (!files.includes(file)) return;
      const module = server.moduleGraph.getModuleById(GUIDES);
      if (module) server.moduleGraph.invalidateModule(module);
      server.ws.send({ type: "full-reload" });
      return [];
    },
  };
}

// The manifest lives beside the gallery, one folder up.
export default defineConfig({
  plugins: [react(), docs()],
  server: { fs: { allow: [".."] } },
});
