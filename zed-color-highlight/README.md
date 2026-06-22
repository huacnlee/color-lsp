# Zed Color Highlight

[![Zed Extension](https://img.shields.io/badge/-Zed_Extension-blue?style=flat&logo=zedindustries&logoColor=%23FFFFFF&logoSize=auto&labelColor=%23111111&color=%23084CCF)](https://zed.dev/extensions/color-highlight)

Highlight colors in your editor based on color-lsp by LSP document colors.

<img width="1285" alt="SCR-20250626-oney" src="https://github.com/user-attachments/assets/a1a211d9-dec4-440b-8c74-848d7b03ff52" />

## Usage

Add the following to your Zed settings:

```json
{
  "lsp_document_colors": "background"
}
```

Due to a missing feature in the Zed Extension system, it's only enabled by default on a predefined set of languages. (if you want this changed, please upvote the [upstream feature request](https://github.com/zed-industries/zed/discussions/45360))

You can configure Zed to enable ColorLSP on more languages. For example, to enable it on Markdown, you can add to your config:

```jsonc
{
  // ...
  "languages": {
    "Markdown": {
      "language-servers": ["...", "color-lsp"],
    },
  },
  // ...
}
```

## License

MIT
