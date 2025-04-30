use crate::element_handler::Element;
use crate::node_util::{get_node_children, get_node_content};
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
pub(crate) fn table_handler(element: Element) -> Option<String> {
    let content = element.content.trim();
    if content.is_empty() {
        return None;
    }

    // Extract table rows
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut has_thead = false;

    // Extract rows and headers from the table structure
    if let NodeData::Element { .. } = &element.node.data {
        for child in get_node_children(element.node) {
            if let NodeData::Element { name, .. } = &child.data {
                let tag_name = name.local.as_ref();

                match tag_name {
                    "thead" => {
                        has_thead = true;
                        headers = extract_row_cells(&child, "th");
                        if headers.is_empty() {
                            headers = extract_row_cells(&child, "td");
                        }
                    }
                    "tbody" | "tfoot" => {
                        for row_node in get_node_children(&child) {
                            if let NodeData::Element { name, .. } = &row_node.data {
                                if name.local.as_ref() == "tr" {
                                    let row_cells = extract_row_cells(&row_node, "td");
                                    if !row_cells.is_empty() {
                                        rows.push(row_cells);
                                    }
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
        return Some(concat_strings!("\n\n", content, "\n\n"));
    }

    // Determine the number of columns by finding the max column count
    let num_columns = if headers.is_empty() {
        rows.iter().map(|row| row.len()).max().unwrap_or(0)
    } else {
        headers.len()
    };

    if num_columns == 0 {
        return Some(concat_strings!("\n\n", content, "\n\n"));
    }

    // Build the Markdown table
    let mut table_md = String::from("\n\n");

    // Add header row if available
    if !headers.is_empty() {
        table_md.push_str("| ");
        for (i, header) in headers.iter().enumerate() {
            if i < num_columns {
                table_md.push_str(&normalize_cell_content(header));
                table_md.push_str(" |");
            }
        }
        table_md.push('\n');

        // Add separator row
        table_md.push_str("| ");
        for _ in 0..num_columns {
            table_md.push_str("--- |");
        }
        table_md.push('\n');
    }

    // Add data rows
    for row in rows {
        table_md.push_str("| ");
        for i in 0..num_columns {
            if i < row.len() {
                table_md.push_str(&normalize_cell_content(&row[i]));
            }
            table_md.push_str(" |");
        }
        table_md.push('\n');
    }

    table_md.push('\n');
    Some(table_md)
}

/// Extract cells from a row node
fn extract_row_cells(row_node: &Rc<markup5ever_rcdom::Node>, cell_tag: &str) -> Vec<String> {
    let mut cells = Vec::new();

    for cell_node in get_node_children(row_node) {
        if let NodeData::Element { name, .. } = &cell_node.data {
            if name.local.as_ref() == cell_tag {
                let cell_content = get_node_content(&cell_node).trim().to_string();
                cells.push(cell_content);
            }
        }
    }

    cells
}

/// Normalize cell content for Markdown table representation
fn normalize_cell_content(content: &str) -> String {
    let content = content.replace('\n', " ").replace('\r', "");
    let content = content.trim();

    content.to_string()
}

