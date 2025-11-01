use crate::element_handler::{Element, serialize_element};
use crate::node_util::{get_node_children, get_node_content, get_node_tag_name};
use crate::options::TranslationMode;
use crate::serialize_if_faithful;
use crate::text_util::concat_strings;
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
pub(crate) fn table_handler(element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    // All child table elements must be markdown translated to markdown
    // translate the table in faithful mode.
    if element.options.translation_mode == TranslationMode::Faithful && !element.markdown_translated
    {
        return (Some(serialize_element(&element)), false);
    }

    let content = element.content.trim();
    if content.is_empty() {
        return (None, true);
    }

    // Extract table rows
    let mut captions: Vec<String> = Vec::new();
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut has_thead = false;

    // Extract rows and headers from the table structure
    if let NodeData::Element { .. } = &element.node.data {
        for child in get_node_children(element.node) {
            if let NodeData::Element { name, .. } = &child.data {
                let tag_name = name.local.as_ref();

                match tag_name {
                    "caption" => {
                        captions.push(get_node_content(&child).trim().to_string());
                    }
                    "thead" => {
                        let tr = child
                            .children
                            .borrow()
                            .iter()
                            .find(|it| get_node_tag_name(it).is_some_and(|tag| tag == "tr"))
                            .cloned();

                        let row_node = match tr {
                            Some(tr) => tr,
                            None => child,
                        };

                        has_thead = true;
                        headers = extract_row_cells(&row_node, "th");
                        if headers.is_empty() {
                            headers = extract_row_cells(&row_node, "td");
                        }
                    }
                    "tbody" | "tfoot" => {
                        for row_node in get_node_children(&child) {
                            if let NodeData::Element { name, .. } = &row_node.data
                                && name.local.as_ref() == "tr"
                            {
                                // If no thead is found, use the first th row as header
                                if !has_thead && headers.is_empty() {
                                    headers = extract_row_cells(&row_node, "th");
                                    has_thead = !headers.is_empty();

                                    if has_thead {
                                        continue;
                                    }
                                }

                                let row_cells = extract_row_cells(&row_node, "td");
                                if !row_cells.is_empty() {
                                    rows.push(row_cells);
                                }
                            }
                        }
                    }
                    "tr" => {
                        // If no thead is found, use the first row as headers
                        if !has_thead && headers.is_empty() {
                            headers = extract_row_cells(&child, "th");
                            if headers.is_empty() {
                                let cells = extract_row_cells(&child, "td");
                                if !cells.is_empty() {
                                    headers = cells;
                                }
                            }
                            has_thead = !headers.is_empty();
                        } else {
                            let row_cells = extract_row_cells(&child, "td");
                            if !row_cells.is_empty() {
                                rows.push(row_cells);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // If we didn't find any rows or cells, just return the content as-is
    if rows.is_empty() && headers.is_empty() {
        return (Some(concat_strings!("\n\n", content, "\n\n")), true);
    }

    // Determine the number of columns by finding the max column count
    let num_columns = if headers.is_empty() {
        rows.iter().map(|row| row.len()).max().unwrap_or(0)
    } else {
        headers.len()
    };

    if num_columns == 0 {
        return (Some(concat_strings!("\n\n", content, "\n\n")), true);
    }

    // Build the Markdown table
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
    (Some(table_md), true)
}

/// Extract cells from a row node
fn extract_row_cells(row_node: &Rc<markup5ever_rcdom::Node>, cell_tag: &str) -> Vec<String> {
    let mut cells = Vec::new();

    for cell_node in get_node_children(row_node) {
        if let NodeData::Element { name, .. } = &cell_node.data
            && name.local.as_ref() == cell_tag
        {
            let cell_content = get_node_content(&cell_node).trim().to_string();
            cells.push(cell_content);
        }
    }

    cells
}

/// Normalize cell content for Markdown table representation
fn normalize_cell_content(content: &str) -> String {
    let content = content
        .replace('\n', " ")
        .replace('\r', "")
        .replace('|', "&#124;");
    content.trim().to_string()
}

fn format_row_padded(row: &[String], num_columns: usize, col_widths: &[usize]) -> String {
    let mut line = String::from("|");
    for (i, col_width) in col_widths.iter().enumerate().take(num_columns) {
        let cell = row
            .get(i)
            .map(|s| normalize_cell_content(s))
            .unwrap_or_default();
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
