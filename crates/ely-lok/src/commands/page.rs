//! Spreadsheet rows: printing and protection.

use super::{Arg, Check, Command, command};

/// Print layout, breaks and protection (TABLE-14, 15).
pub(super) const ROWS: &[Command] = &[
    command(
        "print.area",
        ".uno:ChangePrintArea",
        &[
            Arg::Loose("PrintArea"),
            Arg::Loose("PrintRepeatRow"),
            Arg::Loose("PrintRepeatCol"),
        ],
        Check::Undoable,
    ),
    command(
        "page.setup",
        ".uno:PageFormatDialog",
        &[Arg::Text("Page")],
        Check::Undoable,
    ),
    command("break.row", ".uno:InsertRowBreak", &[], Check::Undoable),
    command(
        "break.column",
        ".uno:InsertColumnBreak",
        &[],
        Check::Undoable,
    ),
    command(
        "break.row_remove",
        ".uno:DeleteRowbreak",
        &[],
        Check::Undoable,
    ),
    command(
        "break.column_remove",
        ".uno:DeleteColumnbreak",
        &[],
        Check::Undoable,
    ),
    command(
        "sheet.protect",
        ".uno:Protect",
        &[Arg::Text("Protection")],
        Check::Undoable,
    ),
    command(
        "cells.lock",
        ".uno:CellProtection",
        &[Arg::Bool("CellProtection")],
        Check::Reports,
    ),
];
