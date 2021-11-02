# mdbook-note

to install :

`cargo install --color=always --path .`

then to use, in book.toml : 

```
[preprocessor.note]
renderers = ["html"]
name = "notes"
cleanup_only = false
```