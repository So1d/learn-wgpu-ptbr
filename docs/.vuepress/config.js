import { defineUserConfig } from "vuepress";
import { viteBundler } from "@vuepress/bundler-vite";
import { defaultTheme } from "@vuepress/theme-default";

import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineUserConfig({
  base: "/learn-wgpu-ptbr/",
  title: "Aprenda Wgpu",
  public: "res",
  bundler: viteBundler({
    viteOptions: {
      plugins: [wasm(), topLevelAwait()],
    },
  }),
  theme: defaultTheme({
    navbar: [
      {
        text: "Início",
        link: "/",
      },
      {
        text: "Iniciante",
        collapsable: false,
        children: [
          "/beginner/tutorial1-window/",
          "/beginner/tutorial2-surface/",
          "/beginner/tutorial3-pipeline/",
          "/beginner/tutorial4-buffer/",
          "/beginner/tutorial5-textures/",
          "/beginner/tutorial6-uniforms/",
          "/beginner/tutorial7-instancing/",
          "/beginner/tutorial8-depth/",
          "/beginner/tutorial9-models/",
        ],
      },
      {
        text: "Intermediário",
        collapsable: false,
        children: [
          "/intermediate/tutorial10-lighting/",
          "/intermediate/tutorial11-normals/",
          "/intermediate/tutorial12-camera/",
          "/intermediate/tutorial13-hdr/",
        ],
      },
      {
        text: "Pipelines de Computação",
        collapsable: true,
        children: ["/compute/introduction/", "/compute/sorting/"],
      },
      {
        text: "Showcase",
        collapsable: true,
        children: [
          "/showcase/",
          "/showcase/mipmaps/",
          "/showcase/stencil/",
          "/showcase/windowless/",
          "/showcase/gifs/",
          "/showcase/pong/",
          "/showcase/alignment/",
          // '/showcase/compute/',
        ],
      },
    ],
  }),
});
