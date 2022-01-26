// @ts-check

const lightCodeTheme = require("prism-react-renderer/themes/github");
const darkCodeTheme = require("prism-react-renderer/themes/dracula");

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: "Gofer",
  tagline: "Simple, opinionated, container-focused, continuous thing do-er.",
  url: "https://clintjedwards.github.io",
  baseUrl: "/gofer/",
  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "error",
  favicon: "/img/favicon.ico",
  trailingSlash: false,
  organizationName: "clintjedwards", // Usually your GitHub org/user name.
  projectName: "gofer", // Usually your repo name.

  presets: [
    [
      "@docusaurus/preset-classic",
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve("./sidebars.js"),
          editUrl: "https://github.com/clintjedwards/gofer/edit/main/website/",
        },
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      navbar: {
        title: "Gofer",
        logo: {
          alt: "Gofer",
          src: "/img/logo-hq.png",
        },
        items: [
          {
            type: "doc",
            docId: "intro",
            position: "left",
            label: "Documentation",
          },
          {
            href: "https://github.com/clintjedwards/gofer",
            label: "GitHub",
            position: "right",
          },
        ],
      },
      footer: {
        style: "dark",
        links: [
          {
            title: "Docs",
            items: [
              {
                label: "Getting Started",
                to: "/docs/getting-started/installing-gofer",
              },
            ],
          },
          {
            title: "More",
            items: [
              {
                label: "GitHub",
                href: "https://github.com/clintjedwards/gofer",
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Clint Edwards.`,
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
        additionalLanguages: ["hcl"],
      },
      algolia: {
        appId: "BPSEOI9BHY",
        apiKey: "8d08c07353dede8823c0a364ff8cbe74",
        indexName: "gofer",
        contextualSearch: true,
      },
    }),
};

module.exports = config;
