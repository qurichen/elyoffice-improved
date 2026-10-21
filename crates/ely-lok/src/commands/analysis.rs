//! Spreadsheet rows: pivot tables and charts.

use super::{Arg, Check, Command, command};

/// Pivot tables and charts (TABLE-10, 11).
pub(super) const ROWS: &[Command] = &[
    command(
        "pivot.make",
        ".uno:DataPilotExec",
        &[Arg::Text("Pivot")],
        Check::Undoable,
    ),
    command(
        "pivot.refresh",
        ".uno:RecalcPivotTable",
        &[],
        Check::Finished,
    ),
    command(
        "pivot.delete",
        ".uno:DeletePivotTable",
        &[],
        Check::Undoable,
    ),
    command(
        "chart.insert",
        ".uno:InsertObjectChart",
        &[
            Arg::Text("RangeList"),
            Arg::Bool("InNewTable"),
            Arg::Text("Chart"),
        ],
        Check::Undoable,
    ),
    command(
        "chart.change",
        ".uno:ChangeChartData",
        &[
            Arg::Text("Name"),
            Arg::Text("Range"),
            Arg::Bool("ColHeaders"),
            Arg::Bool("RowHeaders"),
            Arg::Text("Chart"),
        ],
        Check::Undoable,
    ),
];
