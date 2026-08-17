use crate::element_handler::element_util::serialize_element;
use crate::element_handler::{Element, HandlerResult, Handlers};
use crate::node_util::{get_node_tag_name, get_parent_node};
use crate::options::TranslationMode;
use crate::serialize_if_faithful;
use crate::text_util::{TrimDocumentWhitespace, concat_strings};
use markup5ever_rcdom::NodeData;
use std::rc::Rc;

/// Handler for table elements.
///
/// Converts HTML tables to Markdown tables using the pipe syntax:
/// ```text
/// | Header1 | Header2 |
/// | ------- | ------- |
/// | Cell1   | Cell2   |
/// ```
pub(crate) fn table_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    if handlers.options().translation_mode == TranslationMode::Pure
        && (!has_explicit_headers(element.node) || is_inside_table_cell(element.node))
    {
        return handlers.fallback(element);
    }

    let ExtractedTable {
        captions,
        headers,
        rows,
        all_children_translated,
    } = extract_table_content(handlers, element.node);

    if handlers.options().translation_mode == TranslationMode::Faithful && !all_children_translated
    {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }

    if rows.is_empty() && headers.is_empty() {
        let content = handlers.walk_children(element.node).content;
        let content = content.trim_matches('\n');
        if content.is_empty() {
            return None;
        }
        return Some(concat_strings!("\n\n", content, "\n\n").into());
    }

    let num_columns = headers
        .len()
        .max(rows.iter().map(|row| row.len()).max().unwrap_or(0));

    let mut table_md = String::from("\n\n");

    for caption in captions {
        table_md.push_str(&format!("{caption}\n"));
    }

    let col_widths = compute_column_widths(&headers, &rows, num_columns);

    if !headers.is_empty() {
        table_md.push_str(&format_row_padded(&headers, num_columns, &col_widths));
        table_md.push_str(&format_separator_padded(num_columns, &col_widths));
    }
    for row in rows {
        table_md.push_str(&format_row_padded(&row, num_columns, &col_widths));
    }

    table_md.push('\n');
    Some(table_md.into())
}

struct ExtractedTable {
    captions: Vec<String>,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    all_children_translated: bool,
}

fn extract_table_content(
    handlers: &dyn Handlers,
    table_node: &Rc<markup5ever_rcdom::Node>,
) -> ExtractedTable {
    let mut table = ExtractedTable {
        captions: Vec::new(),
        headers: Vec::new(),
        rows: Vec::new(),
        all_children_translated: true,
    };
    let mut has_thead = false;

    for child in table_node.children.borrow().iter() {
        let NodeData::Element { name, .. } = &child.data else {
            continue;
        };

        match name.local.as_ref() {
            "caption" => {
                if let Some(result) = handlers.handle(child) {
                    table
                        .captions
                        .push(result.content.trim_document_whitespace().to_string());
                }
            }
            "thead" => {
                has_thead = true;
                extract_thead(handlers, child, &mut table);
            }
            "tbody" | "tfoot" => extract_section_rows(handlers, child, &mut table, &mut has_thead),
            "tr" => extract_direct_row(handlers, child, &mut table, &mut has_thead),
            _ => {}
        }
    }

    table
}

fn extract_thead(
    handlers: &dyn Handlers,
    thead_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
) {
    let children = thead_node.children.borrow();
    let row_node = children
        .iter()
        .find(|node| get_node_tag_name(node).is_some_and(|tag| tag == "tr"))
        .unwrap_or(thead_node);

    let (headers, translated) = extract_row_cells(handlers, row_node, "th");
    table.headers = headers;
    table.all_children_translated &= translated;
    if table.headers.is_empty() {
        let (headers, translated) = extract_row_cells(handlers, row_node, "td");
        table.headers = headers;
        table.all_children_translated &= translated;
    }
}

fn extract_section_rows(
    handlers: &dyn Handlers,
    section_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
    has_thead: &mut bool,
) {
    for row_node in section_node.children.borrow().iter() {
        if get_node_tag_name(row_node) != Some("tr") {
            continue;
        }

        if !*has_thead && table.headers.is_empty() {
            let (headers, translated) = extract_row_cells(handlers, row_node, "th");
            table.headers = headers;
            table.all_children_translated &= translated;
            *has_thead = !table.headers.is_empty();
            if *has_thead {
                continue;
            }
        }

        let (cells, translated) = extract_row_cells(handlers, row_node, "td");
        table.all_children_translated &= translated;
        if !cells.is_empty() {
            table.rows.push(cells);
        }
    }
}

fn extract_direct_row(
    handlers: &dyn Handlers,
    row_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
    has_thead: &mut bool,
) {
    if !*has_thead && table.headers.is_empty() {
        let (headers, translated) = extract_row_cells(handlers, row_node, "th");
        table.headers = headers;
        table.all_children_translated &= translated;
        if table.headers.is_empty() {
            let (headers, translated) = extract_row_cells(handlers, row_node, "td");
            table.headers = headers;
            table.all_children_translated &= translated;
        }
        *has_thead = !table.headers.is_empty();
    } else {
        let (cells, translated) = extract_row_cells(handlers, row_node, "td");
        table.all_children_translated &= translated;
        if !cells.is_empty() {
            table.rows.push(cells);
        }
    }
}

fn has_explicit_headers(node: &Rc<markup5ever_rcdom::Node>) -> bool {
    fn visit(node: &Rc<markup5ever_rcdom::Node>, is_root: bool) -> bool {
        for child in node.children.borrow().iter() {
            if let NodeData::Element { name, .. } = &child.data {
                let tag_name = name.local.as_ref();
                if !is_root && tag_name == "table" {
                    continue;
                }
                if matches!(tag_name, "th" | "thead") {
                    return true;
                }
            }

            if visit(child, false) {
                return true;
            }
        }

        false
    }

    visit(node, true)
}

fn is_inside_table_cell(node: &Rc<markup5ever_rcdom::Node>) -> bool {
    let mut current = get_parent_node(node);

    while let Some(parent) = current {
        if get_node_tag_name(&parent).is_some_and(|tag| matches!(tag, "td" | "th")) {
            return true;
        }
        current = get_parent_node(&parent);
    }

    false
}

/// Extract cells from a row node
fn extract_row_cells(
    handlers: &dyn Handlers,
    row_node: &Rc<markup5ever_rcdom::Node>,
    cell_tag: &str,
) -> (Vec<String>, bool) {
    let mut cells = Vec::new();
    let mut all_translated = true;

    for cell_node in row_node.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &cell_node.data
            && name.local.as_ref() == cell_tag
        {
            let Some(res) = handlers.handle(cell_node) else {
                continue;
            };
            if !res.markdown_translated {
                all_translated = false;
            }
            let cell_content = normalize_cell_content(res.content.trim_document_whitespace());
            cells.push(cell_content);
        }
    }

    (cells, all_translated)
}

/// Normalize cell content for Markdown table representation
fn normalize_cell_content(content: &str) -> String {
    let content = content
        .replace('\n', " ")
        .replace('\r', "")
        .replace('|', "&#124;");
    content.trim_document_whitespace().to_string()
}

fn format_row_padded(row: &[String], num_columns: usize, col_widths: &[usize]) -> String {
    let mut line = String::from("|");
    for (i, col_width) in col_widths.iter().enumerate().take(num_columns) {
        let cell = row.get(i).map(String::as_str).unwrap_or_default();
        let pad = col_width.saturating_sub(cell.chars().count());
        line.push_str(&concat_strings!(" ", cell, " ".repeat(pad), " |"));
    }
    line.push('\n');
    line
}

fn format_separator_padded(num_columns: usize, col_widths: &[usize]) -> String {
    let mut line = String::from("|");
    for (_, col_width) in col_widths.iter().enumerate().take(num_columns) {
        line.push_str(&concat_strings!(" ", "-".repeat(*col_width), " |"));
    }
    line.push('\n');
    line
}

fn compute_column_widths(
    headers: &[String],
    rows: &[Vec<String>],
    num_columns: usize,
) -> Vec<usize> {
    let mut widths = vec![0; num_columns];
    for (i, header) in headers.iter().enumerate() {
        widths[i] = header.chars().count();
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(num_columns) {
            let len = cell.chars().count();
            if len > widths[i] {
                widths[i] = len;
            }
        }
    }
    widths
}
