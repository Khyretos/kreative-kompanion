use super::*;

fn tmp() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("kk-docs-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

async fn view_of(name: &str, bytes: &[u8]) -> String {
    let d = tmp();
    let p = d.join("file.bin");
    std::fs::write(&p, bytes).unwrap();
    let v = view(name, &p, bytes.len() as u64, &bytes[..bytes.len().min(8192)]).await;
    std::fs::remove_dir_all(&d).ok();
    v.text
}

#[test]
fn xml_entities_and_tags() {
    assert_eq!(unescape("a &amp; b &lt;c&gt; &#65;&#x42; &quot;q&quot;"), "a & b <c> AB \"q\"");
    assert_eq!(xml_text("<a x=\"1\">one <b>two</b></a>&amp;"), "one two&");
}

#[test]
fn a_word_body_gives_paragraphs_headings_and_tabs() {
    let xml = r#"<w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Intro</w:t></w:r></w:p>
        <w:p><w:r><w:t xml:space="preserve">Fish </w:t></w:r><w:r><w:t>&amp; chips</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>now</w:t></w:r></w:p>
        <w:p/><w:p><w:r><w:t>one</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>two</w:t></w:r></w:p>
        <w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>Big</w:t></w:r></w:p></w:body>"#;
    assert_eq!(docx_text(xml), "# Intro\nFish & chips\tnow\n\none two\n# Big\n");
}

#[test]
fn a_sheet_gives_tab_separated_rows_with_cells_in_their_columns() {
    let shared = vec!["Name".to_string(), "Qty".to_string(), "R&D".to_string()];
    let xml = r#"<sheetData>
        <row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row>
        <row r="2"><c r="A2" t="inlineStr"><is><t>apple</t></is></c><c r="D2"><v>12.5</v></c></row>
        <row r="3"><c r="A3"/></row>
        <row r="4"><c r="B4" t="b"><v>1</v></c><c r="C4" t="s"><v>2</v></c></row></sheetData>"#;
    assert_eq!(sheet_rows(xml, &shared), "Name\tQty\napple\t\t\t12.5\n\tTRUE\tR&D\n");
    assert_eq!(column_index("A"), 0);
    assert_eq!(column_index("AB"), 27);
}

#[test]
fn slides_odf_and_rtf_give_their_text() {
    assert_eq!(slide_text(r#"<a:p><a:r><a:t>Title</a:t></a:r></a:p><a:p><a:r><a:t>one</a:t></a:r><a:br/><a:r><a:t>two</a:t></a:r></a:p>"#), "Title\none\ntwo\n");
    let odt = r#"<text:h text:outline-level="2">Part</text:h><text:p>a<text:s text:c="2"/>b<text:tab/>c</text:p><draw:page draw:name="Cover"><text:p>Hi</text:p></draw:page>"#;
    assert_eq!(odf_text(odt), "## Part\na  b\tc\n\n=== slide 1: Cover ===\nHi\n");
    let ods = r#"<table:table table:name="Data"><table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell table:number-columns-repeated="3"/></table:table-row><table:table-row><table:table-cell/></table:table-row><table:table-row><table:table-cell><text:p>b</text:p></table:table-cell><table:table-cell><text:p>c</text:p></table:table-cell></table:table-row></table:table>"#;
    assert_eq!(ods_text(ods), "=== sheet Data ===\na\nb\tc\n\n");
    assert_eq!(rtf_text(r"{\rtf1\ansi{\fonttbl{\f0 Arial;}}{\*\generator X;}\f0 Hello \b world\b0\par Second line\'e9 \u8364? end}"), "Hello world\nSecond lineé € end");
}

#[test]
fn pdf_pages_are_marked_and_a_single_page_is_left_alone() {
    assert_eq!(mark_pages("one page"), "one page");
    assert_eq!(mark_pages("a\nb\x0c\nc\x0c"), "=== page 1 ===\na\nb\n=== page 2 ===\nc\n");
}

#[test]
fn ffprobe_json_becomes_a_few_lines() {
    let v: Value = serde_json::from_str(r#"{"format":{"format_long_name":"QuickTime / MOV","duration":"3725.4","bit_rate":"2500000","tags":{"TITLE":"Holiday","artist":"Me"}},
        "streams":[{"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"r_frame_rate":"30000/1001"},{"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2}]}"#).unwrap();
    let s = probe_summary(&v).unwrap();
    assert!(s.starts_with("Format: QuickTime / MOV, duration 1:02:05, 2500 kbit/s\n"), "{s}");
    assert!(s.contains("Title: Holiday") && s.contains("Artist: Me"), "{s}");
    assert!(s.contains("Video: h264, 1920x1080, 29.97 fps") && s.contains("Audio: aac, 48000 Hz, 2 channel(s)"), "{s}");
    assert!(probe_summary(&serde_json::json!({})).is_none());
}

#[test]
fn files_with_a_reader_are_known_by_name() {
    for n in ["a.docx", "B.XLSX", "c.pptx", "d.odt", "e.epub", "f.rtf", "g.zip", "h.tar.gz", "i.tgz", "j.log.gz"] { assert!(has_reader(n), "{n}") }
    for n in ["a.txt", "b.exe", "c.png", "noext"] { assert!(!has_reader(n), "{n}") }
}

#[tokio::test]
async fn office_files_give_their_text() {
    let docx = stored_zip(&[("word/document.xml", br#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>Plan</w:t></w:r></w:p><w:p><w:r><w:t>Do it.</w:t></w:r></w:p>"#)]);
    let t = view_of("plan.docx", &docx).await;
    assert!(t.starts_with("Word document: plan.docx ("), "{t}");
    assert!(t.contains("## Plan\nDo it.\n"), "{t}");

    let xlsx = stored_zip(&[
        ("xl/workbook.xml", br#"<sheets><sheet name="Sales" sheetId="1" r:id="rId1"/><sheet name="Costs" sheetId="2" r:id="rId2"/></sheets>"#),
        ("xl/sharedStrings.xml", br#"<sst><si><t>Item</t></si><si><t>Price</t></si></sst>"#),
        ("xl/worksheets/sheet2.xml", br#"<sheetData><row><c r="A1"><v>9</v></c></row></sheetData>"#),
        ("xl/worksheets/sheet1.xml", br#"<sheetData><row><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row></sheetData>"#),
    ]);
    let t = view_of("books.xlsx", &xlsx).await;
    assert!(t.starts_with("Excel workbook: books.xlsx ("), "{t}");
    assert!(t.contains("=== sheet Sales ===\nItem\tPrice\n") && t.contains("=== sheet Costs ===\n9\n"), "{t}");

    let pptx = stored_zip(&[
        ("ppt/slides/slide2.xml", br#"<a:p><a:r><a:t>Second</a:t></a:r></a:p>"#),
        ("ppt/slides/slide1.xml", br#"<a:p><a:r><a:t>First</a:t></a:r></a:p>"#),
        ("ppt/notesSlides/notesSlide1.xml", br#"<a:p><a:r><a:t>say hello</a:t></a:r></a:p>"#),
    ]);
    let t = view_of("deck.pptx", &pptx).await;
    assert!(t.contains("=== slide 1 ===\nFirst\nNotes: say hello\n") && t.contains("=== slide 2 ===\nSecond\n"), "{t}");
    assert!(t.find("slide 1").unwrap() < t.find("slide 2").unwrap());

    let odt = stored_zip(&[("content.xml", br#"<text:p>from open document</text:p>"#)]);
    assert!(view_of("a.odt", &odt).await.contains("OpenDocument text: a.odt"));
    let epub = stored_zip(&[("OEBPS/ch1.xhtml", b"<html><body><p>Once upon a time</p></body></html>")]);
    assert!(view_of("b.epub", &epub).await.contains("=== chapter: OEBPS/ch1.xhtml ===\nOnce upon a time"));
}

#[tokio::test]
async fn archives_give_a_listing_and_compressed_text_its_text() {
    let zip = stored_zip(&[("src/main.rs", b"fn main() {}"), ("README.md", b"hello")]);
    let t = view_of("code.zip", &zip).await;
    assert!(t.starts_with("ZIP archive: code.zip ("), "{t}");
    assert!(t.contains("2 entries") && t.contains("src/main.rs  (12 B)") && t.contains("README.md  (5 B)"), "{t}");
    // A renamed or unknown zip is still listed.
    assert!(view_of("thing.xyz", &zip).await.contains("src/main.rs"));

    let mut tar = tar::Builder::new(Vec::new());
    let mut h = tar::Header::new_gnu();
    h.set_size(5);
    h.set_cksum();
    tar.append_data(&mut h, "dir/a.txt", &b"hello"[..]).unwrap();
    let raw = tar.into_inner().unwrap();
    let t = view_of("x.tar", &raw).await;
    assert!(t.contains("Tar archive") && t.contains("1 entries") && t.contains("dir/a.txt  (5 B)"), "{t}");
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut gz, &raw).unwrap();
    let t = view_of("x.tar.gz", &gz.finish().unwrap()).await;
    assert!(t.contains("Compressed tar archive") && t.contains("dir/a.txt"), "{t}");

    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut gz, b"2026-01-01 ERROR boom\n").unwrap();
    let t = view_of("app.log.gz", &gz.finish().unwrap()).await;
    assert!(t.contains("Compressed text file: app.log.gz") && t.contains("ERROR boom"), "{t}");
}

#[tokio::test]
async fn a_broken_document_and_an_unknown_binary_still_get_a_description() {
    let t = view_of("bad.docx", b"PK\x03\x04 not really a zip at all").await;
    assert!(t.starts_with("Word document: bad.docx") && t.contains("could not be read"), "{t}");
    let mut elf = b"\x7fELF\0\0\0\0".to_vec();
    elf.extend_from_slice(b"\0\0\0/lib64/ld-linux-x86-64.so.2\0\0libc.so.6\0\x01\x02hi\0");
    let t = view_of("prog", &elf).await;
    assert!(t.starts_with("ELF executable: prog (") && t.contains("/lib64/ld-linux-x86-64.so.2") && t.contains("libc.so.6"), "{t}");
    assert!(!t.contains("\nhi\n"), "short runs are noise");
    let t = view_of("blob.dat", &[0u8, 1, 2, 3, 200, 201]).await;
    assert!(t.contains("Binary file: blob.dat") && t.contains("no readable text found"), "{t}");
    let t = view_of("s.rtf", br"{\rtf1 Hello\par world}").await;
    assert!(t.contains("RTF document: s.rtf") && t.contains("Hello\nworld"), "{t}");
}
