use std::io::{Write, Result};

#[derive(Default)]
pub struct Sheet {
    pub id: usize,
    pub name: String,
    pub columns: Vec<Column>,
    max_row_id: usize,
}

#[derive(Default)]
pub struct Row {
    pub cells: Vec<Cell>,
    row_index: usize,
}

pub trait ToCellValue {
    fn to_cell_value(&self) -> CellValue;
}

impl ToCellValue for bool {
    fn to_cell_value(&self) -> CellValue {
        CellValue::Bool(self.to_owned())
    }
}

impl ToCellValue for f64 {
    fn to_cell_value(&self) -> CellValue {
        CellValue::Number(self.to_owned())
    }
}

impl ToCellValue for String {
    fn to_cell_value(&self) -> CellValue {
        CellValue::String(self.to_owned())
    }
}

impl<'a> ToCellValue for &'a str {
    fn to_cell_value(&self) -> CellValue {
        CellValue::String(self.to_owned().to_owned())
    }
}

impl Row {
    pub fn new() -> Row {
        Row { ..Default::default() }
    }

    pub fn add_cell<T>(&mut self, column_index: u8, value: T) where T: ToCellValue + Sized {
        self.cells.push(Cell { column_index: column_index, value: value.to_cell_value() });
    }

    pub fn write(&self, writer: &mut Write) -> Result<()> {
        let head = format!("<row r=\"{}\">\n", self.row_index);
        writer.write_all(head.as_bytes())?;
        for c in self.cells.iter() {
            c.write(self.row_index, writer)?;
        }
        writer.write_all("\n</row>\n".as_bytes())
    }
}

pub enum CellValue {
    Bool(bool),
    Number(f64),
    String(String),
}

fn write_value(cv: &CellValue, ref_id: String, writer: &mut Write) -> Result<()> {
    match cv {
        &CellValue::Bool(b) => {
            let v = match b {
                true => 1,
                false => 0,
            };
            let s = format!("<c r=\"{}\" t=\"b\"><v>{}</v></c>", ref_id, v);
            writer.write_all(s.as_bytes())?;
        },
        &CellValue::Number(num) => {
            let s = format!("<c r=\"{}\" ><v>{}</v></c>", ref_id, num);
            writer.write_all(s.as_bytes())?;
        },
        &CellValue::String(ref s) => {
            let s = format!("<c r=\"{}\" t=\"str\"><v>{}</v></c>", ref_id, s);
            writer.write_all(s.as_bytes())?;
        }
    }
    Ok(())
}

pub struct Cell {
    pub column_index: u8,
    pub value: CellValue,
}

impl Cell {
    fn write(&self, row_index: usize, writer: &mut Write) -> Result<()> {
        let ref_id = format!("{}{}", column_letter(self.column_index), row_index);
        write_value(&self.value, ref_id, writer)
    }
}

pub fn column_letter(column_index: u8) -> String {
    format!("{}", ('A' as u8 + (column_index - 1)) as char)
}

#[derive(Default)]
pub struct Column {
    pub width: f32,
    pub custom_width: f32,
}

impl Sheet {
    pub fn new(sheet_name: &str) -> Sheet {
        Sheet {
            id: 1,
            name: sheet_name.to_owned(),
            ..Default::default()
        }
    }

    pub fn add_column(&mut self, column: Column) {
        self.columns.push(column)
    }

/*
    fn write<W, F>(&self, writer: &mut W, write_data: F) -> Result<()>
        where F: FnOnce(&mut W) -> Result<()> + Sized, W: Write + Sized {
        self.write_head(writer)?;

        self.write_data_begin(writer)?;

        write_data(writer)?;

        self.write_data_end(writer)?;
        self.close(writer)
    }

*/
    fn write_row<W>(&mut self, writer: &mut W, mut row: Row) -> Result<()>
        where W: Write + Sized {
        self.max_row_id += 1;
        row.row_index = self.max_row_id;
        row.write(writer)
    }

    fn write_empty_rows(&mut self, rows: usize) {
        self.max_row_id += rows;
    }

    fn write_head(&self, writer: &mut Write) -> Result<()> {
        let header = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
        xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
        "#;
        writer.write_all(header.as_bytes())?;
        /*
                let dimension = format!("<dimension ref=\"A1:{}{}\"/>", column_letter(self.dimension.columns), self.dimension.rows);
                writer.write_all(dimension.as_bytes())?;
        */

        if self.columns.is_empty() {
            return Ok(())
        }

        writer.write_all("\n<cols>\n".as_bytes())?;
        let mut i = 1;
        for col in self.columns.iter() {
            writer.write_all(format!("<col min=\"{}\" max=\"{}\" width=\"{}\" customWidth=\"{}\"/>\n", &i, &i, col.width, col.custom_width).as_bytes())?;
            i += 1;
        }
        writer.write_all("</cols>\n".as_bytes())
    }

    fn write_data_begin(&self, writer: &mut Write) -> Result<()> {
        writer.write_all("\n<sheetData>\n".as_bytes())
    }

    fn write_data_end(&self, writer: &mut Write) -> Result<()> {
        writer.write_all("\n</sheetData>\n".as_bytes())
    }

    fn close(&self, writer: &mut Write) -> Result<()> {
        let foot = "</worksheet>\n";
        writer.write_all(foot.as_bytes())
    }
}

pub struct SheetWriter<'a, 'b, Writer> where 'b: 'a, Writer: 'b {
    sheet: &'a mut Sheet,
    writer: &'b mut Writer,
}

impl<'a, 'b, Writer> SheetWriter<'a, 'b, Writer>
where Writer: Write + Sized
{
    pub fn new(sheet: &'a mut Sheet, writer: &'b mut Writer) -> SheetWriter<'a, 'b, Writer> {
        SheetWriter { sheet: sheet, writer: writer }
    }

    pub fn append_row(&mut self, row: Row) -> Result<()> {
        self.sheet.write_row(self.writer, row)
    }
    pub fn append_empty_rows(&mut self, rows: usize) {
        self.sheet.write_empty_rows(rows)
    }

    pub fn write<F>(&mut self, write_data: F) -> Result<()>
        where F: FnOnce(&mut SheetWriter<Writer>) -> Result<()> + Sized {
        self.sheet.write_head(self.writer)?;

        self.sheet.write_data_begin(self.writer)?;

        write_data(self)?;

        self.sheet.write_data_end(self.writer)?;
        self.sheet.close(self.writer)
    }
}