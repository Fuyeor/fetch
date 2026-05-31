**Fetch** is the reference implementation of the **Search Patterns Protocol (SPP)**. 

The [Search Patterns Protocol](https://fuyeor.com/t/560) is an experimental indexing standard based on pattern matching. Unlike traditional crawling methods, SPP is designed for controlled environments, specifically catering to enterprises and organizations that manage multiple web properties and require a unified, high-precision cross-site search infrastructure.

SPP is architecture-agnostic. Whether a site is a Client-Side Rendered (CSR) app, a Multi-Page Application (MPA), or built on any backend stack, it can be seamlessly indexed provided that the Search Patterns Protocol is implemented.

### Objectives

- **Unified Search for Organizations**: Facilitate the creation of a centralized search engine across diverse and numerous websites within a single ecosystem.
- **Pattern Matching Feasibility**: Test and implement a deterministic pattern-based indexing method, moving away from redundant link-traversal.
- **Environmental Responsibility**: Drastically reduce the carbon footprint of search operations by eliminating inefficient "brute-force" crawling.

### Examples

**1. Search Patterns (The Protocol Configuration)**

> This file defines how different sections of your site are structured and where the corresponding data resides, like traditional Sitemap Index.

```fer
// .well-known/fuyeor/search-patterns
version = 1.0
patterns = [
  {
    path = `/@{username}`
    priority = 0.7
    items = `https://fuyeor.com/.well-known/fuyeor/users`
  }
  {
    path = `/article/{id}`
    priority = 0.6
    items = `https://fuyeor.com/.well-known/fuyeor/articles`
  }
  {
    path = `{locale}/product/{id}/{slug}`
    priority = 0.6
    items = `https://fuyeor.com/.well-known/fuyeor/products`
  }
]
```

**2. Item Lists (Sitemap)**

> These files function as the **Sitemap** within the Search Patterns Protocol. They contain arrays of values mapped to the placeholders defined in the `search-patterns`.

If a pattern contains only **one** placeholder (e.g., `/article/{id}`), you can use a simple string array. The values are automatically mapped to that unique placeholder.

```fer
// .well-known/fuyeor/articles
[
  `101`, // Resolves to: example.com/article/101
  `102`,

  // Specific entries can be overridden with metadata like priority and lastmod.
  {
    id = `103`
    priority = 1.0
    lastmod = `2026-05-28`
  } // Resolves to: example.com/article/103
]
```

If a pattern contains **two or more** placeholders (e.g., `{locale}/product/{id}/{slug}`), the entries **must** be an array of objects specifying each variable.

```fer
// .well-known/fuyeor/products
// Pattern: `{locale}/product/{id}/{slug}`
[
  {
    locale = `en`
    id = `500`
    slug = `park-short`
    priority = 1.0
    lastmod = `2026-05-28`
  } // Resolves to: example.com/en/product/500/park-short
]
```