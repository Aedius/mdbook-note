use mdbook::book::{Book, Chapter, SectionNumber};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use mdbook::BookItem;
use regex::{Captures, Regex, RegexBuilder};
use std::collections::HashMap;

pub struct Note {
    regex: Regex,
}

#[derive(Eq, PartialEq, Debug, Clone)]
struct Extract {
    key: Vec<String>,
    val: String,
}

struct Extracts {
    name: String,
    list: Vec<Extract>,
}

impl Note {
    pub fn new() -> Note {
        let re = RegexBuilder::new(
            r"\{\{#note ?(?P<key>[^}]*)}}(?P<val>[^\{]*)\{\{#note end}}",
        )
        .multi_line(true)
        .dot_matches_new_line(true)
        .build()
        .unwrap();

        Note { regex: re }
    }

    fn parse_chapter(&self, chapter: &Chapter) -> Vec<Extract> {
        let mut res = vec![];

        let mut find_key: HashMap<String, bool> = HashMap::new();

        for cap in self.regex.captures_iter(chapter.content.as_str()) {
            let key = capture(&cap, "key");

            let mut keys: Vec<String> = key
                .clone()
                .split('|')
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| s != &"".to_string())
                .collect();
            keys.reverse();

            if !find_key.contains_key(&*key) {
                res.push(Extract {
                    key: keys.clone(),
                    val: format!("### {}", chapter.name),
                });
                find_key.insert(key, true);
            }
            res.push(Extract {
                key: keys,
                val: capture(&cap, "val"),
            })
        }

        res
    }

    fn clean_chapter(&self, mut chapter: Chapter) -> Chapter {
        let content = chapter.content.clone();

        let new_content = self.regex.replace_all(&content, "$val");

        chapter.content = new_content.to_string();

        chapter
    }
}

fn capture(cap: &Captures, k: &str) -> String {
    match cap.name(k) {
        Some(res) => res.as_str().trim().to_string(),
        None => "".to_string(),
    }
}

#[cfg(test)]
mod extract_tests {
    use super::*;

    #[test]
    fn test_extract_inline() {
        let chapter = Chapter {
            name: "some name".to_string(),
            content: "some outer content {{#note my_key}}inside contente{{#note end}} other outer content".to_string(),
            number: None,
            sub_items: vec![],
            path: None,
            source_path: None,
            parent_names: vec![],
        };

        let note = Note::new();

        assert_eq!(
            note.parse_chapter(&chapter),
            vec![
                Extract {
                    key: vec!["my_key".to_string()],
                    val: "### some name".to_string(),
                },
                Extract {
                    key: vec!["my_key".to_string()],
                    val: "inside contente".to_string(),
                }
            ]
        )
    }

    #[test]
    fn test_extract_multiline() {
        let chapter = Chapter {
            name: "some name".to_string(),
            content: "some outer content
            {{#note my_key}}
            inside contente
            {{#note end}}
            other outer content"
                .to_string(),
            number: None,
            sub_items: vec![],
            path: None,
            source_path: None,
            parent_names: vec![],
        };

        let note = Note::new();

        assert_eq!(
            note.parse_chapter(&chapter),
            vec![
                Extract {
                    key: vec!["my_key".to_string()],
                    val: "### some name".to_string(),
                },
                Extract {
                    key: vec!["my_key".to_string()],
                    val: "inside contente".to_string(),
                }
            ]
        )
    }

    #[test]
    fn test_extract_multiline_multicapture() {
        let chapter = Chapter {
            name: "some name".to_string(),
            content: "some outer content
{{#note my_key| my sub key}}
inside contente split
{{#note end}}
other outer content
blablabla
{{#note my key 2}}
other content
split
{{#note end}}
{{#note}}
some global note
{{#note end}}
{{#note my key 2}}
my other key 2
{{#note end}}
end
"
            .to_string(),
            number: None,
            sub_items: vec![],
            path: None,
            source_path: None,
            parent_names: vec![],
        };

        let note = Note::new();

        assert_eq!(
            note.parse_chapter(&chapter),
            vec![
                Extract {
                    key: vec!["my sub key".to_string(), "my_key".to_string()],
                    val: "### some name".to_string(),
                },
                Extract {
                    key: vec!["my sub key".to_string(), "my_key".to_string()],
                    val: "inside contente split".to_string(),
                },
                Extract {
                    key: vec!["my key 2".to_string()],
                    val: "### some name".to_string(),
                },
                Extract {
                    key: vec!["my key 2".to_string()],
                    val: "other content\nsplit".to_string(),
                },
                Extract {
                    key: vec![],
                    val: "### some name".to_string(),
                },
                Extract {
                    key: vec![],
                    val: "some global note".to_string(),
                },
                Extract {
                    key: vec!["my key 2".to_string()],
                    val: "my other key 2".to_string(),
                },
            ]
        )
    }
}

impl Preprocessor for Note {
    fn name(&self) -> &str {
        "note"
    }

    fn run(&self, ctx: &PreprocessorContext, book: Book) -> Result<Book, Error> {
        let mut name = "note".to_string();

        // In testing we want to tell the preprocessor to blow up by setting a
        // particular config value
        if let Some(nop_cfg) = ctx.config.get_preprocessor(self.name()) {
            match nop_cfg.get("name") {
                None => {}
                Some(value) => {
                    name = value.as_str().unwrap().to_string();
                }
            }
        }

        let mut extracts: Vec<Extract> = vec![];

        let mut new_book = Book::new();

        for item in book.iter() {
            let new_item = match item {
                BookItem::Chapter(chapter) => {
                    let mut ext = self.parse_chapter(chapter);
                    extracts.append(&mut ext);
                    let clean = self.clean_chapter(chapter.clone());
                    BookItem::Chapter(clean)
                }
                BookItem::Separator => BookItem::Separator,
                BookItem::PartTitle(title) => BookItem::PartTitle(title.to_string()),
            };
            new_book.push_item(new_item);
        }

        if extracts.is_empty() {
            return Ok(book);
        }

        let note_chapter = generate_chapter(extracts, name, vec![], vec![99]);

        new_book.push_item(note_chapter);

        // we *are* a no-op preprocessor after all
        Ok(new_book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}

fn generate_chapter(
    extracts: Vec<Extract>,
    name: String,
    parent: Vec<String>,
    section: Vec<u32>,
) -> Chapter {
    let mut extract_by_key = HashMap::new();

    let mut current_name = parent.clone();
    current_name.push(name.clone());

    let mut chapter = Chapter {
        name: name.clone(),
        content: format!("## {}", current_name.join(" / ")),
        number: Some(SectionNumber(section.clone())),
        sub_items: vec![],
        path: Some(name.parse().unwrap()),
        source_path: None,
        parent_names: parent.clone(),
    };

    let mut parent = parent;
    parent.push(name);

    for extract in extracts {
        let mut local = extract.clone();

        match local.key.pop() {
            None => {
                if !chapter.content.is_empty() {
                    chapter.content = format!("{}\n\n{}", chapter.content, extract.val);
                } else {
                    chapter.content = extract.val;
                }
            }
            Some(k) => {
                let val = extract_by_key.entry(k).or_insert_with(Vec::new);
                val.push(local);
            }
        }
    }

    let mut extract_to_sort = vec![];
    for (name, list) in extract_by_key.into_iter() {
        let extract = Extracts { name, list };
        extract_to_sort.push(extract);
    }

    extract_to_sort.sort_by(|a, b| a.name.cmp(&b.name));

    let mut i = 1;
    for extract in extract_to_sort {
        let mut section = section.clone();
        section.push(i);

        let new_chapter = generate_chapter(extract.list, extract.name, parent.clone(), section);

        chapter.sub_items.push(BookItem::Chapter(new_chapter));

        i += 1;
    }

    chapter
}

#[cfg(test)]
mod generate_tests {
    use super::*;
    use mdbook::book::SectionNumber;

    #[test]
    fn test_generate_chapter() {
        let extracts = vec![
            Extract {
                key: vec!["b".to_string()],
                val: "content b".to_string(),
            },
            Extract {
                key: vec!["a1".to_string(), "a".to_string()],
                val: "content a1".to_string(),
            },
            Extract {
                key: vec![],
                val: "note content".to_string(),
            },
            Extract {
                key: vec!["a2".to_string(), "a".to_string()],
                val: "content a2".to_string(),
            },
            Extract {
                key: vec!["a2".to_string(), "a".to_string()],
                val: "content a2 2".to_string(),
            },
        ];

        let chapter = Chapter {
            name: "note".to_string(),
            content: "## note\n\nnote content".to_string(),
            number: Some(SectionNumber(vec![1])),
            sub_items: vec![
                BookItem::Chapter(Chapter {
                    name: "a".to_string(),
                    content: "## note / a".to_string(),
                    number: Some(SectionNumber(vec![1, 1])),
                    sub_items: vec![
                        BookItem::Chapter(Chapter {
                            name: "a1".to_string(),
                            content: "## note / a / a1\n\ncontent a1".to_string(),
                            number: Some(SectionNumber(vec![1, 1, 1])),
                            sub_items: vec![],
                            path: Some("a1".parse().unwrap()),
                            source_path: None,
                            parent_names: vec!["note".to_string(), "a".to_string()],
                        }),
                        BookItem::Chapter(Chapter {
                            name: "a2".to_string(),
                            content: "## note / a / a2\n\ncontent a2\n\ncontent a2 2".to_string(),
                            number: Some(SectionNumber(vec![1, 1, 2])),
                            sub_items: vec![],
                            path: Some("a2".parse().unwrap()),
                            source_path: None,
                            parent_names: vec!["note".to_string(), "a".to_string()],
                        }),
                    ],
                    path: Some("a".parse().unwrap()),
                    source_path: None,
                    parent_names: vec!["note".to_string()],
                }),
                BookItem::Chapter(Chapter {
                    name: "b".to_string(),
                    content: "## note / b\n\ncontent b".to_string(),
                    number: Some(SectionNumber(vec![1, 2])),
                    sub_items: vec![],
                    path: Some("b".parse().unwrap()),
                    source_path: None,
                    parent_names: vec!["note".to_string()],
                }),
            ],
            path: Some("note".parse().unwrap()),
            source_path: None,
            parent_names: vec![],
        };

        assert_eq!(
            generate_chapter(extracts, "note".to_string(), vec![], vec![1]),
            chapter
        )
    }
}
