# Prax ORM Documentation

The Prax ORM documentation website, built with [Astro](https://astro.build/), Tailwind CSS 4,
and the custom `@pegasusheavy/tailswatch` **oxide** theme. Pages are static — code samples are
highlighted at build time with Prism (including a custom `prax` schema language grammar).

## Prerequisites

This project uses [pnpm](https://pnpm.io/) as the package manager.

```bash
# Install dependencies
pnpm install
```

## Development server

```bash
pnpm dev
```

Open `http://localhost:4321/`. The site reloads automatically when you modify source files.

## Project structure

```text
src/
├── components/
│   ├── CodeBlock.astro   # Syntax-highlighted code block with copy button
│   └── Sidebar.astro     # Docs sidebar (renders nav.ts)
├── layouts/
│   └── DocsLayout.astro  # Page shell: sidebar, mobile header, <head> meta
├── lib/
│   └── prism.ts          # Prism setup + custom prax/prisma grammar
├── pages/                # One .astro file per route (schema/, queries/, database/, ...)
├── styles/
│   └── global.css        # Tailwind + oxide theme + code block styles
└── nav.ts                # Sidebar navigation structure
```

## Authoring pages

Each page wraps its content in `DocsLayout` and uses `CodeBlock` for code samples:

```astro
---
import DocsLayout from '../layouts/DocsLayout.astro';
import CodeBlock from '../components/CodeBlock.astro';

const example = `model User {
    id Int @id @auto
}`;
---

<DocsLayout title="My Page - Prax ORM">
  <article class="max-w-4xl mx-auto px-6 py-12">
    <CodeBlock code={example} lang="prax" filename="prax/schema.prax" />
  </article>
</DocsLayout>
```

Notes:

- `lang` supports `prax`, `prisma`, `rust`, `toml`, `bash`, `sql`, `json`, `yaml`, `typescript`,
  `graphql`. Omit it for auto-detection.
- In template text, literal `{`/`}` must be escaped (`&#123;`/`&#125;`) or wrapped in a string
  expression (`{'...'}`) — Astro treats them as expression delimiters.
- In frontmatter template literals, escape `` \` `` and `\${` sequences.
- To add a page to the sidebar, add an entry to `src/nav.ts`.

## Building

```bash
pnpm build
```

Outputs the static site to `dist/`. Preview the production build locally with `pnpm preview`.
