use super::*;
use std::io::Cursor;

fn lines_of(text: &str) -> Vec<(usize, String)> {
    let mut v = Vec::new();
    scan(Cursor::new(text.as_bytes().to_vec()), |n, l| { v.push((n, l.to_string())); true }).unwrap();
    v
}

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("kk-read-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn doc(dir: &Path, name: &str, text: &str) -> Doc {
    let p = dir.join(format!("{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&p, text).unwrap();
    Doc { name: name.into(), kind: "text".into(), original: Some(p.clone()), text: Some(p), size: text.len() as u64 }
}

fn numbered(n: usize) -> String {
    (1..=n).map(|i| format!("line number {i} of the file")).collect::<Vec<_>>().join("\n")
}

#[test]
fn sizes_follow_the_window_and_an_unknown_window_is_a_small_one() {
    let small = Budget::new(None, true);
    assert_eq!((small.window, small.inline, small.read), (8_192, 6_144, 3_072));
    assert_eq!(Budget::new(Some(500), true).window, 8_192, "a nonsense window counts as unknown");
    let tiny = Budget::new(Some(4_096), false);
    assert_eq!((tiny.inline, tiny.read, tiny.tools), (3_072, 1_536, false));
    let mid = Budget::new(Some(32_768), true);
    assert_eq!((mid.inline, mid.read), (24_576, 12_288));
    let big = Budget::new(Some(1_000_000), true);
    assert_eq!((big.inline, big.read), (100_000, 48_000), "capped, so a huge window does not mean huge results");
    let plan = Plan::new(None, true);
    assert_eq!(plan.left, plan.budget.inline);
}

#[test]
fn lines_are_numbered_from_one_and_line_ends_do_not_matter() {
    assert_eq!(lines_of("a\r\nb\n\nlast"), vec![(1, "a".into()), (2, "b".into()), (3, "".into()), (4, "last".into())]);
    assert_eq!(lines_of("one\n"), vec![(1, "one".into())]);
    assert!(lines_of("").is_empty());
}

#[test]
fn a_long_line_is_split_so_line_numbers_always_work() {
    let v = lines_of(&format!("{}\nshort", "x".repeat(2_500)));
    assert_eq!(v.iter().map(|(n, l)| (*n, l.len())).collect::<Vec<_>>(), vec![(1, 1000), (2, 1000), (3, 500), (4, 5)]);
    // 70,000 two-byte characters in one line cross the 64 KiB reading chunk in the middle of a character.
    let v = lines_of(&"é".repeat(70_000));
    assert_eq!(v.iter().map(|(_, l)| l.chars().count()).sum::<usize>(), 70_000);
    assert!(v.iter().all(|(_, l)| !l.contains('\u{FFFD}')), "no character is torn apart");
}

#[test]
fn a_page_is_cut_by_size_and_says_where_to_go_on() {
    let text = numbered(100);
    let p = read_page(Cursor::new(text.clone().into_bytes()), 1, 200).unwrap();
    assert!(p.more && p.last >= 5 && p.last < 10, "{}", p.last);
    assert!(p.text.starts_with("line number 1 of the file\n") && p.text.len() <= 200 + 30);
    let next = read_page(Cursor::new(text.clone().into_bytes()), p.last + 1, 200).unwrap();
    assert!(next.text.starts_with(&format!("line number {} of", p.last + 1)), "the next page continues where the first stopped");
    let end = read_page(Cursor::new(text.clone().into_bytes()), 98, 10_000).unwrap();
    assert!(!end.more && end.last == 100 && end.text.lines().count() == 3);
    let none = read_page(Cursor::new(text.into_bytes()), 500, 100).unwrap();
    assert!(none.text.is_empty());
    let one = read_page(Cursor::new(b"abcdefghij\nsecond".to_vec()), 1, 3).unwrap();
    assert_eq!((one.text.as_str(), one.more), ("abcdefghij\n", true), "a page always has one line");
}

#[test]
fn the_overview_finds_headings_definitions_pages_and_columns() {
    let md = "# Title\ntext\n## Part one\nmore\n#nothing\n=== sheet Data ===\n=== page 1 ===\n=== page 2 ===\n";
    let o = overview(Cursor::new(md.as_bytes().to_vec()), "doc").unwrap();
    assert_eq!((o.lines, o.pages), (8, 2));
    assert_eq!(o.outline, vec![(1, "# Title".to_string()), (3, "## Part one".to_string()), (6, "=== sheet Data ===".to_string())]);
    let rs = "use x;\n# not a heading\npub async fn run() {}\nstruct A;\n    fn inner() {}\nlet fn_name = 1;\n";
    let o = overview(Cursor::new(rs.as_bytes().to_vec()), "rs").unwrap();
    assert_eq!(o.outline.iter().map(|(n, _)| *n).collect::<Vec<_>>(), vec![3, 4, 5]);
    let o = overview(Cursor::new(b"id,name,qty\n1,a,2\n".to_vec()), "csv").unwrap();
    assert_eq!(o.first, "id,name,qty");
    let t = overview_text("d.csv", "csv", &o, 3_000);
    assert!(t.starts_with("File: d.csv, 2 lines\nColumns: id,name,qty\n"), "{t}");
    let many: String = (0..100).map(|i| format!("# H{i}\n")).collect();
    let o = overview(Cursor::new(many.into_bytes()), "doc").unwrap();
    assert_eq!((o.outline.len(), o.outline_more), (OUTLINE_MAX, 40));
    let t = overview_text("m.md", "md", &o, 300);
    assert!(t.len() < 300 && t.contains("more (use search_attachment)"), "a small room shows few headings: {t}");
}

#[test]
fn search_lines_find_phrases_and_all_words_and_merge_context() {
    let text = "alpha beta\ngamma\nBeta ALPHA again\ndelta\nepsilon\nzeta\nalpha\n";
    let (total, hits) = find(Cursor::new(text.as_bytes().to_vec()), "alpha", 10).unwrap();
    assert_eq!((total, hits), (3, vec![1, 3, 7]));
    let (total, hits) = find(Cursor::new(text.as_bytes().to_vec()), "beta alpha", 10).unwrap();
    assert_eq!((total, hits), (2, vec![1, 3]), "all words, any order, any case");
    let (_, first) = find(Cursor::new(text.as_bytes().to_vec()), "alpha", 2).unwrap();
    assert_eq!(first, vec![1, 3]);
    assert_eq!(find(Cursor::new(text.as_bytes().to_vec()), "  ", 5).unwrap().0, 0);
    assert_eq!(merge(&[1, 3, 7], 1), vec![(1, 4), (6, 8)]);
    assert_eq!(merge(&[5], 0), vec![(5, 5)]);
    assert_eq!(merge(&[2, 4], 1), vec![(1, 5)]);
}

#[test]
fn the_first_read_has_a_summary_and_each_read_says_where_to_continue() {
    let d = tmp();
    let text = format!("# Report\n{}\n## Findings\n{}", numbered(120), numbered(5));
    let docs = vec![doc(&d, "report.md", &text)];
    let r = read_tool(&docs, Some("report.md"), &json!({}), 1_500).unwrap();
    assert!(r.starts_with("File: report.md, 127 lines\nContents (line: heading):\n  1: # Report\n  122: ## Findings\n"), "{r}");
    assert!(r.contains("line number 1 of the file"), "{r}");
    assert!(r.contains("[Lines 1-") && r.contains("To continue: read_attachment name=\"report.md\" from_line="), "{r}");
    let from: usize = r.rsplit("from_line=").next().unwrap().trim_end_matches(']').parse().unwrap();
    let r2 = read_tool(&docs, Some("report.md"), &json!({"from_line": from}), 1_500).unwrap();
    assert!(!r2.starts_with("File:"), "only the first read has the summary");
    assert!(r2.contains(&format!("[Lines {from}-")), "{r2}");
    let end = read_tool(&docs, None, &json!({"from_line": 125}), 1_500).unwrap();
    assert!(end.contains("the end of the file") && end.contains("line number 5 of the file"), "{end}");
    let past = read_tool(&docs, None, &json!({"from_line": 999}), 1_500).unwrap();
    assert!(past.contains("has 127 lines") && past.contains("nothing from line 999"), "{past}");
    assert!(r.len() < 1_500 + 200, "a read stays near its room: {}", r.len());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_file_is_picked_by_name_or_the_model_is_told_the_names() {
    let d = tmp();
    let mut pic = doc(&d, "cat.png", "x");
    pic.kind = "image".into();
    pic.text = None;
    let docs = vec![doc(&d, "Notes.md", "one"), doc(&d, "notes-old.md", "two"), pic, doc(&d, "Notes.md", "newest")];
    assert_eq!(pick(&docs, Some("notes.md")).map(|d| d.size), Ok(6), "an exact name, the newest of equal names");
    assert_eq!(pick(&docs, Some("old")).map(|d| d.name.clone()), Ok("notes-old.md".to_string()), "part of a name");
    let e = pick(&docs, None).unwrap_err();
    assert!(e.starts_with("Which file?") && e.contains("- Notes.md (text, 3 B)") && e.contains("- cat.png (picture)"), "{e}");
    let e = pick(&docs, Some("missing.pdf")).unwrap_err();
    assert!(e.starts_with("No attached file is called missing.pdf"), "{e}");
    assert_eq!(pick(&docs[..1], None).map(|d| d.name.clone()), Ok("Notes.md".to_string()), "one file needs no name");
    let e = pick(&docs[2..3], None).unwrap_err();
    assert!(e.contains("None of the attached files has text"), "{e}");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn searching_shows_matches_with_context_and_line_numbers() {
    let d = tmp();
    let text = format!("{}\nthe ERROR is here\n{}\nanother error appears\n{}", numbered(30), numbered(30), numbered(30));
    let docs = vec![doc(&d, "app.log", &text), doc(&d, "other.txt", "all fine here")];
    let r = search_tool(&docs, Some("app.log"), &json!({"query": "error"}), 3_000).unwrap();
    assert!(r.starts_with("\"error\" in app.log: 2 matching line(s)."), "{r}");
    assert!(r.contains("31* the ERROR is here\n") && r.contains("30: line number 30 of the file\n") && r.contains("32: line number 1 of the file\n"), "{r}");
    assert!(r.contains("62* another error appears\n") && r.contains("--\n"), "{r}");
    let all = search_tool(&docs, None, &json!({"query": "error", "context": 0}), 3_000).unwrap();
    assert!(all.contains("31* the ERROR is here\n--\n62* another error") && all.contains("No line of other.txt has \"error\"."), "{all}");
    assert!(search_tool(&docs, None, &json!({"query": " "}), 3_000).is_err());
    let wide = vec![doc(&d, "wide.txt", &(1..=100).map(|i| format!("needle {i} {}", "w".repeat(110))).collect::<Vec<_>>().join("\n"))];
    let small = search_tool(&wide, Some("wide.txt"), &json!({"query": "needle", "context": 5}), 1_200).unwrap();
    assert!(small.len() < 1_500 && small.contains("more matches"), "a small room cuts the result and says so: {}", small.len());
    assert!(small.contains("100 matching line(s), the first 15 shown"), "{small}");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_text_file_inside_an_archive_can_be_read() {
    let d = tmp();
    let zip = crate::attach_docs::stored_zip(&[("src/main.rs", b"fn main() {\n}\n"), ("img/a.png", &[0x89, 0x50, 0, 1, 2])]);
    let p = d.join("a.bin");
    std::fs::write(&p, &zip).unwrap();
    let z = Doc { name: "code.zip".into(), kind: "file".into(), original: Some(p), text: None, size: zip.len() as u64 };
    assert_eq!(member_text(&z, "src/main.rs").unwrap().1, "fn main() {\n}\n");
    assert_eq!(member_text(&z, "main.rs").unwrap().0, "src/main.rs", "the end of the path is enough");
    assert!(member_text(&z, "img/a.png").unwrap_err().contains("is not text"));
    assert!(member_text(&z, "nope.txt").unwrap_err().contains("is not in the archive"));

    let mut tar = tar::Builder::new(Vec::new());
    let mut h = tar::Header::new_gnu();
    h.set_size(5);
    h.set_cksum();
    tar.append_data(&mut h, "dir/a.txt", &b"hello"[..]).unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut gz, &tar.into_inner().unwrap()).unwrap();
    let p = d.join("b.bin");
    std::fs::write(&p, gz.finish().unwrap()).unwrap();
    let t = Doc { name: "x.tar.gz".into(), kind: "file".into(), original: Some(p), text: None, size: 1 };
    assert_eq!(member_text(&t, "dir/a.txt").unwrap().1, "hello");

    // Reading a member goes through the same pages.
    let text_view = d.join("a.txt");
    std::fs::write(&text_view, "listing").unwrap();
    let mut z2 = z.clone();
    z2.text = Some(text_view);
    let r = read_tool(&[z2], Some("code.zip"), &json!({"member": "main.rs"}), 1_500).unwrap();
    assert!(r.starts_with("File: src/main.rs in code.zip, 2 lines") && r.contains("fn main() {") && r.contains("the end of the file"), "{r}");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn the_tools_are_listed_under_both_formats() {
    assert!(is_tool("read_attachment") && is_tool("search_attachment") && !is_tool("read_file"));
    assert_eq!(tools().len(), 2);
    assert_eq!(specs()[1]["function"]["parameters"]["required"], json!(["query"]));
    for d in tools().iter().map(|t| t["description"].as_str().unwrap().len()).chain(specs().iter().map(|t| t["function"]["description"].as_str().unwrap().len())) {
        assert!(d <= 300, "the chat tool list keeps 300 characters of a description: {d}");
    }
}
