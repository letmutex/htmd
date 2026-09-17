use crate::Context;
use crate::element_handler::element_util::serialize_element_result;
use crate::element_handler::element_util::serialize_if_extra_attrs_or_inline;
use crate::element_handler::{Element, HandlerResult, Handlers};
use crate::options::TranslationMode;
use crate::util::{
    node::{get_node_tag_name, get_parent_node},
    text::{TrimDocumentWhitespace, concat_strings, frame_as_block},
};
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
pub(crate) fn table_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    // A GFM table is a CommonMark block, so the extraction below always runs in
    // a block context.
    serialize_if_extra_attrs_or_inline!(handlers, element, 0);
    if handlers.options().translation_mode == TranslationMode::Pure
        && (!has_explicit_headers(element.node) || is_inside_table_cell(element.node))
    {
        return handlers.fallback(element);
    }

    let ExtractedTable {
        captions,
        mut headers,
        rows,
        all_children_translated,
    } = extract_table_content(handlers, element.node);

    if handlers.options().translation_mode == TranslationMode::Faithful && !all_children_translated
    {
        return Some(serialize_element_result(handlers, element));
    }

    if rows.is_empty() && headers.is_empty() {
        let content = handlers.walk_children_content(element.node, element.context);
        let content = content.trim_matches('\n');
        if content.is_empty() {
            return None;
        }
        return Some(frame_as_block(content).into());
    }

    let num_columns = headers
        .len()
        .max(rows.iter().map(|row| row.len()).max().unwrap_or(0));

    // A [GFM table](https://github.github.com/gfm/#tables-extension-) has no
    // headerless form: the delimiter row which makes the block a table has to
    // follow a header row. Only pure mode reaches here without one, the row
    // widths below having sent faithful mode to HTML, and it has no fallback to
    // write in place of the header row it invents.
    if headers.is_empty() {
        headers = vec![String::new(); num_columns];
    }

    let mut table_md = String::from("\n\n");

    table_md.push_str(&captions);

    let col_widths = compute_column_widths(&headers, &rows, num_columns);

    table_md.push_str(&format_row_padded(&headers, num_columns, &col_widths));
    table_md.push_str(&format_separator_padded(num_columns, &col_widths));
    for row in rows {
        table_md.push_str(&format_row_padded(&row, num_columns, &col_widths));
    }

    table_md.push('\n');
    Some(table_md.into())
}

struct ExtractedTable {
    /// The caption blocks, each already terminated by a blank line.
    captions: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    all_children_translated: bool,
}

fn extract_table_content(
    handlers: &dyn Handlers,
    table_node: &Rc<markup5ever_rcdom::Node>,
) -> ExtractedTable {
    let mut table = ExtractedTable {
        captions: String::new(),
        headers: Vec::new(),
        rows: Vec::new(),
        all_children_translated: true,
    };
    let mut has_thead = false;
    let mut has_tbody = false;

    for child in table_node.children.borrow().iter() {
        let NodeData::Element { name, .. } = &child.data else {
            // The parser foster-parents a table's stray text out of it, so the
            // only text reaching here is the whitespace between its elements. A
            // comment has nowhere to go in a Markdown table.
            table.all_children_translated &= matches!(child.data, NodeData::Text { .. });
            continue;
        };

        // A table only builds CommonMark in a block context, so its caption,
        // sections, rows and cells all sit in one.
        match name.local.as_ref() {
            "caption" => {
                if let Some(result) = handlers.handle(child, Context::BLOCK) {
                    // A caption which only HTML can express takes the whole
                    // table with it: written as an HTML block of its own it
                    // would land outside any table, where the "in body"
                    // insertion mode drops the start tag as a parse error.
                    table.all_children_translated &= result.markdown_translated;
                    table
                        .captions
                        .push_str(result.content.trim_document_whitespace());
                    // The blank line is what keeps the table a block of its
                    // own. A caption written as a list or a blockquote would
                    // otherwise take the rows below it as lazy continuation
                    // lines, dissolving the table into that block's text.
                    table.captions.push_str("\n\n");
                }
            }
            // A GFM table has one header row, and it comes first. So a second
            // `<thead>` can only overwrite the header the first one gave, and a
            // `<thead>` following a body section can only become that header by
            // moving ahead of it -- a `<tbody>` holding no row included, since
            // the header it gains would precede a body the Markdown has no
            // spelling for.
            "thead" => {
                table.all_children_translated &=
                    has_no_attributes(child) && !has_thead && !has_tbody && table.rows.is_empty();
                has_thead = true;
                extract_thead(handlers, child, &mut table);
            }
            // A GFM table has one body, so a second `<tbody>`'s rows can only
            // join the first, which loses the split between them.
            "tbody" => {
                table.all_children_translated &= has_no_attributes(child) && !has_tbody;
                has_tbody = true;
                extract_section_rows(handlers, child, &mut table, &mut has_thead);
            }
            // A GFM table has one body, so a `<tfoot>`'s rows can only join it,
            // which loses the footer. Faithful mode writes the table as HTML
            // rather than lose it; pure mode has no fallback and takes the join.
            "tfoot" => {
                table.all_children_translated = false;
                extract_section_rows(handlers, child, &mut table, &mut has_thead);
            }
            // The parser wraps a stray `<tr>` in a `<tbody>`, so a row reaches
            // a table as a direct child of one only in a tree built elsewhere
            // and handed to `HtmlToMarkdown::tree_to_markdown`. Read it as a row
            // of the `<tbody>` the parser would have wrapped it in.
            "tr" => extract_row(handlers, child, &mut table, &mut has_thead),
            // A `<colgroup>`, its `<col>`s, and anything else a table may hold
            // have no Markdown spelling at all.
            _ => table.all_children_translated = false,
        }
    }

    // A GFM table is a header row and the rows under it, so a table holding no
    // header row -- one holding nothing at all included -- has no shape a GFM
    // table can take.
    table.all_children_translated &= !table.headers.is_empty();

    // Every row of a GFM table holds the columns its header declares: a shorter
    // row would be written with empty cells the HTML never held, a longer one
    // with a header column the HTML never declared.
    table.all_children_translated &= table
        .rows
        .iter()
        .all(|row| row.len() == table.headers.len());

    table
}

fn extract_thead(
    handlers: &dyn Handlers,
    thead_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
) {
    let children = thead_node.children.borrow();
    let mut row_nodes = children
        .iter()
        .filter(|node| get_node_tag_name(node) == Some("tr"));
    let row_node = row_nodes.next();
    // A GFM table has exactly one header row, so a second `<tr>` here has
    // nowhere to go.
    table.all_children_translated &= row_nodes.next().is_none() && holds_only(thead_node, "tr");

    extract_header_row(handlers, row_node.unwrap_or(thead_node), table);
}

/// Fills `table.headers` from `row_node`, preferring its `th` cells and falling
/// back to its `td` cells when it holds no `th`.
fn extract_header_row(
    handlers: &dyn Handlers,
    row_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
) {
    if take_header_cells(handlers, row_node, table) {
        return;
    }
    // Neither case left gives a header row a GFM table can hold: the `td`
    // fallback promotes data cells to something the HTML did not say they were,
    // and a row holding neither kind of cell leaves no header at all.
    let (headers, _) = extract_row_cells(handlers, row_node, "td");
    table.all_children_translated = false;
    table.headers = headers;
}

/// Makes `row_node`'s `th` cells `table.headers`, reporting whether it held
/// any.
fn take_header_cells(
    handlers: &dyn Handlers,
    row_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
) -> bool {
    let (headers, translated) = extract_row_cells(handlers, row_node, "th");
    if headers.is_empty() {
        return false;
    }
    table.all_children_translated &= translated;
    table.headers = headers;
    true
}

fn extract_section_rows(
    handlers: &dyn Handlers,
    section_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
    has_thead: &mut bool,
) {
    table.all_children_translated &= holds_only(section_node, "tr");

    for row_node in section_node.children.borrow().iter() {
        if get_node_tag_name(row_node) == Some("tr") {
            extract_row(handlers, row_node, table, has_thead);
        }
    }
}

/// Appends `row_node` to `table`, as its header row where it holds `th` cells
/// and no header row has been found yet, and as a body row otherwise.
fn extract_row(
    handlers: &dyn Handlers,
    row_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
    has_thead: &mut bool,
) {
    // A GFM table's header row comes first, so a `<th>` row following a body
    // row cannot become one without reordering the table.
    if !*has_thead
        && table.headers.is_empty()
        && table.rows.is_empty()
        && take_header_cells(handlers, row_node, table)
    {
        *has_thead = true;
        return;
    }

    extract_body_row(handlers, row_node, table);
}

/// Appends `row_node`'s `td` cells to `table.rows`.
fn extract_body_row(
    handlers: &dyn Handlers,
    row_node: &Rc<markup5ever_rcdom::Node>,
    table: &mut ExtractedTable,
) {
    let (cells, translated) = extract_row_cells(handlers, row_node, "td");
    // A row which yields no cells is dropped rather than written as the empty
    // Markdown row it has no content for.
    table.all_children_translated &= translated && !cells.is_empty();
    if !cells.is_empty() {
        table.rows.push(cells);
    }
}

/// Whether every child of `node` is a `tag` element, ignoring text. Another
/// element, or a comment, would be dropped from the Markdown table, which only
/// faithful mode can avoid by writing the table as HTML; text is exempt because
/// the parser foster-parents a table's stray text out of it, leaving only the
/// whitespace between its elements.
fn holds_only(node: &Rc<markup5ever_rcdom::Node>, tag: &str) -> bool {
    node.children
        .borrow()
        .iter()
        .all(|child| match &child.data {
            NodeData::Element { name, .. } => name.local.as_ref() == tag,
            NodeData::Text { .. } => true,
            _ => false,
        })
}

/// Whether `node`'s subtree holds a comment whose text has a `|`, which splits
/// the Markdown row the comment is written into. Neither escape for a `|`
/// reaches inside a comment: a backslash escape is read only in CommonMark
/// text, and a character reference stays the literal characters composing it.
/// Only faithful mode can keep such a comment, by writing the table as HTML.
/// See the "Table cells" section of `unsupported_html.md`.
fn holds_a_comment_with_a_pipe(node: &Rc<markup5ever_rcdom::Node>) -> bool {
    node.children.borrow().iter().any(|child| {
        matches!(&child.data, NodeData::Comment { contents } if contents.contains('|'))
            || holds_a_comment_with_a_pipe(child)
    })
}

/// Whether `node` carries no attributes, which a Markdown table has nowhere to
/// write: a section or row carrying one can only be written as HTML. This is
/// the `num_attrs_allowed` of 0 which `tr_handler` and `table_section_handler`
/// pass to `handle_or_serialize_by_parent`, repeated here because a translated
/// table walks its own sections and rows rather than routing them through those
/// handlers.
fn has_no_attributes(node: &Rc<markup5ever_rcdom::Node>) -> bool {
    match &node.data {
        NodeData::Element { attrs, .. } => attrs.borrow().is_empty(),
        _ => true,
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
    // A Markdown table row has nowhere to write an attribute either, so a `<tr>`
    // carrying one takes the whole table to HTML. Neither can it hold a cell of
    // the other kind -- a GFM row is all header or all body -- nor any of the
    // other elements a `<tr>` may hold, such as a `<script>`.
    let mut all_translated = has_no_attributes(row_node) && holds_only(row_node, cell_tag);

    for cell_node in row_node.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &cell_node.data
            && name.local.as_ref() == cell_tag
        {
            // See `extract_table_content`: a cell of a translated table is
            // always in a block context.
            let Some(res) = handlers.handle(cell_node, Context::BLOCK) else {
                continue;
            };
            // A comment holding a `|` is looked for here rather than over the
            // whole table because a comment anywhere else in one -- between
            // cells, rows or sections -- is already ruled out by `holds_only`,
            // and a `<caption>` takes the table to HTML whatever it holds.
            if !res.markdown_translated || holds_a_comment_with_a_pipe(cell_node) {
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
        .replace('|', "\\|");
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
    for col_width in col_widths.iter().take(num_columns) {
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
    // A column whose every cell is empty measures zero, but a delimiter cell
    // holding no `-` stops the row being a delimiter row, so every column is at
    // least one wide. Padding the cells to the same floor keeps the delimiter
    // row aligned with the rows around it.
    let mut widths = vec![1; num_columns];
    for (i, header) in headers.iter().enumerate() {
        widths[i] = widths[i].max(header.chars().count());
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
