use super::*;

fn write_temp(label: &str, contents: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("living-docs-mermaid-test-{label}-{nanos}.md"));
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn extract_diagrams_captures_multiple_fences_with_body_and_start_line() {
    let path = write_temp(
        "multi",
        "# Doc\n\n```mermaid\nflowchart TD\n  A --> B\n```\n\nMore text.\n\n```mermaid\nerDiagram\n  A ||--o{ B : has\n```\n",
    );
    let diagrams = extract_diagrams(std::slice::from_ref(&path));

    assert_eq!(diagrams.len(), 2);
    assert_eq!(diagrams[0].start_line, 3);
    assert_eq!(diagrams[0].body, "flowchart TD\n  A --> B\n");
    assert_eq!(diagrams[1].start_line, 10);
    assert_eq!(diagrams[1].body, "erDiagram\n  A ||--o{ B : has\n");

    let _ = fs::remove_file(&path);
}

#[test]
fn extract_diagrams_honors_indented_fence_lines() {
    let path = write_temp(
        "indented",
        "- item\n  ```mermaid\n  flowchart TD\n    A --> B\n  ```\n",
    );
    let diagrams = extract_diagrams(std::slice::from_ref(&path));

    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0].start_line, 2);
    assert_eq!(diagrams[0].body, "  flowchart TD\n    A --> B\n");

    let _ = fs::remove_file(&path);
}

#[test]
fn extract_diagrams_drops_an_unterminated_block_at_eof() {
    let path = write_temp("unterminated", "```mermaid\nflowchart TD\n  A --> B\n");
    let diagrams = extract_diagrams(std::slice::from_ref(&path));

    assert!(diagrams.is_empty());

    let _ = fs::remove_file(&path);
}

fn corpus_diagram(body: &str) -> Diagram {
    Diagram {
        file: PathBuf::from("corpus.md"),
        start_line: 1,
        body: body.to_string(),
    }
}

/// ADR 0013 fitness function: the conformance corpus must parse with the
/// exact same accept/reject verdict the prior parser gave — every valid
/// shape accepted, the broken arrow chain rejected. A parity regression
/// in `merman-core` fails this test, not silently degrades `check`.
#[test]
fn validate_all_matches_the_adr_0013_conformance_corpus() {
    let valid = [
        "flowchart TD\n  A[Start] --> B{Decision}\n  B -->|Yes| C[Do the thing]\n  B -->|No| D[Skip it]\n",
        "flowchart LR\n  User -->|shortens| App\n  App -->|redirects| User\n",
        "erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  ORDER ||--|{ LINE_ITEM : contains\n",
        "  flowchart TD\n    A --> B\n",
    ]
    .map(corpus_diagram);
    let failures = validate_all(&valid);
    assert!(
        failures.is_empty(),
        "expected every valid corpus diagram to parse, but {} failed",
        failures.len()
    );

    let invalid = [corpus_diagram("flowchart TD\nA --> --> B\n")];
    let failures = validate_all(&invalid);
    assert_eq!(
        failures.len(),
        1,
        "expected the broken arrow chain to be rejected"
    );
}

#[test]
fn discover_files_with_an_explicit_directory_finds_its_markdown_files() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-mermaid-discover-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("doc.md"), "# Doc\n").unwrap();
    fs::write(dir.join("not-md.txt"), "ignored\n").unwrap();

    let files = discover_files(std::slice::from_ref(&dir));

    assert_eq!(files, vec![dir.join("doc.md")]);

    let _ = fs::remove_dir_all(&dir);
}
