# Chaitanya-0310.github.io

Personal technical blog, built with Jekyll and the
[Chirpy](https://github.com/cotes2020/jekyll-theme-chirpy) theme and hosted on
GitHub Pages at <https://chaitanya-0310.github.io>.

## Writing a post

Add a Markdown file to `_posts/` named `YYYY-MM-DD-slug.md` with front matter:

```yaml
---
title: "Post title"
date: 2026-11-18 10:00:00 -0500
description: "One sentence shown in previews."
categories: [Topic]
tags: [tag-one, tag-two]
---
```

Mermaid code blocks render as diagrams in every post. Pushing to `main`
builds and deploys the site through GitHub Actions.

## Previewing locally

With Docker running:

```bash
docker run --rm -it -v "$PWD":/srv -w /srv -p 4000:4000 ruby:3.4 sh -c "bundle install && bundle exec jekyll serve --host 0.0.0.0"
```

Then open <http://localhost:4000>.
