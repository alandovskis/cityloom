import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

const config: Config = {
  title: "CityLoom",
  tagline: "A street, junction and city-map editor",
  favicon: "img/favicon.svg",

  // Future flags, see https://docusaurus.io/docs/api/docusaurus-config#future
  future: {
    v4: true, // Improve compatibility with the upcoming Docusaurus v4
  },

  // GitHub Pages, deployed by the `docs-deploy` job of .github/workflows/ci.yml on every push to main.
  url: "https://alandovskis.github.io",
  baseUrl: "/cityloom/",
  trailingSlash: false,
  organizationName: "alandovskis",
  projectName: "cityloom",

  onBrokenLinks: "throw",
  onBrokenAnchors: "throw",
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: "throw",
    },
  },

  // The app speaks English and Canadian French, and so does this site.
  i18n: {
    defaultLocale: "en",
    locales: ["en", "fr-CA"],
    localeConfigs: {
      en: { label: "English", htmlLang: "en" },
      "fr-CA": { label: "Français (Canada)", htmlLang: "fr-CA" },
    },
  },

  presets: [
    [
      "classic",
      {
        // The docs are the whole site: they are served from its root.
        docs: {
          path: "docs",
          routeBasePath: "/",
          sidebarPath: "./sidebars.ts",
        },
        // Nothing to say there yet.
        blog: false,
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: "CityLoom",
      logo: {
        alt: "CityLoom",
        src: "img/logo.svg",
        srcDark: "img/logo-dark.svg",
      },
      items: [
        {
          type: "docSidebar",
          sidebarId: "docsSidebar",
          position: "left",
          label: "Documentation",
        },
        {
          type: "localeDropdown",
          position: "right",
        },
        {
          href: "https://github.com/alandovskis/cityloom",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Documentation",
          items: [
            { label: "Introduction", to: "/" },
            { label: "Getting started", to: "/getting-started" },
            { label: "Architecture", to: "/architecture" },
          ],
        },
        {
          title: "Project",
          items: [{ label: "GitHub", href: "https://github.com/alandovskis/cityloom" }],
        },
      ],
      copyright: "Streets © OpenStreetMap contributors. Built with Docusaurus.",
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ["bash", "rust", "toml"],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
