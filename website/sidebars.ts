import type { SidebarsConfig } from "@docusaurus/plugin-content-docs";

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

const sidebars: SidebarsConfig = {
  docsSidebar: [
    "intro",
    "projects",
    "sharing",
    "getting-started",
    "project-layout",
    "testing",
    "test-inventory",
    "coverage-matrix",
    {
      type: "category",
      label: "Architecture",
      items: [
        "architecture/index",
        "architecture/runtime",
        "architecture/data-model-projects",
        "architecture/data-model-streets-junctions",
        "architecture/data-model-server",
      ],
    },
    "issues",
  ],
};

export default sidebars;
