pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Row>,
    pub line_width: usize,
}
pub struct Row {
    pub cells: Vec<String>,
}

impl Table {
    pub fn new() -> Table {
        Table {
            headers: vec![],
            rows: vec![],
            line_width: 100,
        }
    }
    pub fn set_headers(&mut self, headers: Vec<String>) {
        self.headers = headers
    }
    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(Row { cells: row })
    }

    pub fn format(&self) -> String {
        let column_count = std::cmp::max(
            self.headers.len(),
            self.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0),
        );
        let mut column_widths = vec![];
        for i in 0..column_count {
            let column_width = self.headers.get(i).map(|h| h.len()).unwrap_or(0);
            let column_width = std::cmp::max(
                column_width,
                self.rows
                    .iter()
                    .map(|r| r.cells.get(i).map(|c| c.len()).unwrap_or(0))
                    .max()
                    .unwrap_or(0),
            );
            column_widths.push(column_width);
        }

        let mut rows = vec![];
        {
            let mut cells = vec![];
            let mut offset = 0;
            for (i, h) in self.headers.iter().enumerate() {
                let w = column_widths[i];
                offset += w;
                if offset > self.line_width{
                    offset = 0;
                    rows.push(cells.join(" "));
                    cells.clear();
                }
                
                cells.push(format!("{h:>w$}", h = h, w = w));
            }
            rows.push(cells.join(" "));
        }

        for row in &self.rows {
            let mut cells = vec![];
            let mut offset = 0;
            for (i, c) in row.cells.iter().enumerate() {
                let w = column_widths[i];
                offset += w;
                if offset > self.line_width{
                    offset = 0;
                    rows.push(cells.join(" "));
                    cells.clear();
                }

                cells.push(format!("{c:>w$}", c = c, w = w));
            }
            rows.push(cells.join(" "))
        }
        rows.push(rows[0].clone());
        let out = rows.join("\n");

        out
    }
}

macro_rules! row {
    ($($element:expr),* $(,)?) => {
        {
            let mut v = Vec::new();
            $(
                v.push(format!("{}", $element));
            )*
            v
        }
    };
}
