// This file uses esbuild to build our svelte/javascript files.

const esbuild = require("esbuild");
const sveltePlugin = require("esbuild-svelte");

function buildProduction() {
  esbuild
    .build({
      entryPoints: ["src/main.js"],
      bundle: true,
      outfile: "public/build/bundle.js",
      minify: true,
      plugins: [sveltePlugin()],
      logLevel: "info",
    })
    .catch(() => process.exit(1));
}

function buildDevelopment() {
  esbuild
    .build({
      entryPoints: ["src/main.js"],
      bundle: true,
      outfile: "public/build/bundle.js",
      plugins: [sveltePlugin()],
      logLevel: "info",
      watch: {
        onRebuild(error, result) {
          if (error) console.error("watch build failed:", error);
          else console.log("watch build succeeded:", result);
        },
      },
    })
    .catch(() => process.exit(1));
}

function main() {
  let args = process.argv;
  if (args.length > 2 && args[2] === "prod") {
    buildProduction();
    return;
  }
  buildDevelopment();
}

main();
