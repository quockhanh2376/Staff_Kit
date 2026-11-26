"""Writes templates/Asset_Import_Template.xlsx (stdlib only; run: python scripts/make-asset-import-template.py)."""
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

HEADERS = ["Category", "Asset Name", "Asset Code", "Serial Number", "Model", "Brand",
           "Quantity", "Condition", "Staff ID", "Warehouse", "Notes"]
EXAMPLES = [
    ["Laptop", "Lenovo ThinkPad E14", "VNLAP999", "PF-EXAMPLE1", "21SX002QVA", "Lenovo", "", "Spare", "", "IT Store", "Serial: one row per device"],
    ["Laptop", "Lenovo ThinkPad E14", "VNLAP998", "PF-EXAMPLE2", "21SX002QVA", "Lenovo", "", "Spare", "ASWVN001", "", "Staff ID = assigned to that employee"],
    ["Monitor", "Dell P2422H", "VNMON999", "CN-EXAMPLE3", "P2422H", "Dell", "", "New", "", "IT Store", ""],
    ["H Phone", "Jabra Evolve2 65", "VNHP999", "JB-EXAMPLE4", "Evolve2 65", "Jabra", "", "Spare", "", "IT Store", ""],
    ["Mouse", "Logitech M650", "", "", "M650", "Logitech", "20", "", "", "IT Store", "Quantity: no serial, one row per item type"],
    ["Keyboard", "Logitech K120", "", "", "K120", "Logitech", "15", "", "", "IT Store", ""],
]
GUIDE = [
    ["Staff Kit — Asset Import Template"],
    [""],
    ["1. Fill the 'Assets' sheet: one header row, then data. Do not rename the headers."],
    ["2. Category + Asset Name are required on every row."],
    ["3. Serial assets (Laptop, Monitor, H Phone…): one row per device, fill Asset Code + Serial Number, leave Quantity empty or 1."],
    ["4. Quantity assets (Mouse, Keyboard, Headset, USB Type-C Hub…): no serial, fill Quantity."],
    ["5. A Category already in Staff Kit keeps its own type. A new Category follows the file: Serial Number → Serial, Quantity → Quantity."],
    ["6. Staff ID (e.g. ASWVN001) assigns the device to that employee; leave empty for the IT store."],
    ["7. Condition: New, Spare or Broken (empty = Spare)."],
    ["8. Import → Load to staging → fix red rows (Type can be changed per row) → Approve. Approve writes everything or nothing."],
    ["See the 'Example' sheet for sample rows."],
]


def col(i):
    return chr(ord("A") + i)


def sheet_xml(rows, widths, bold_first=True):
    cols = "".join(f'<col min="{i+1}" max="{i+1}" width="{w}" customWidth="1"/>' for i, w in enumerate(widths))
    out = []
    for r, row in enumerate(rows, start=1):
        cells = []
        for c, value in enumerate(row):
            if value == "":
                continue
            style = ' s="1"' if bold_first and r == 1 else ""
            if value.isdigit():
                cells.append(f'<c r="{col(c)}{r}"{style}><v>{value}</v></c>')
            else:
                cells.append(f'<c r="{col(c)}{r}" t="inlineStr"{style}><is><t>{escape(value)}</t></is></c>')
        out.append(f'<row r="{r}">{"".join(cells)}</row>')
    return ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
            f'<cols>{cols}</cols><sheetData>{"".join(out)}</sheetData></worksheet>')


def main():
    widths = [14, 26, 14, 18, 14, 12, 10, 11, 12, 12, 40]
    sheets = [("Assets", sheet_xml([HEADERS], widths)),
              ("Example", sheet_xml([HEADERS] + EXAMPLES, widths)),
              ("Guide", sheet_xml(GUIDE, [120]))]
    ns = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
    files = {
        "[Content_Types].xml": '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            '<Default Extension="xml" ContentType="application/xml"/>'
            '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
            '<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>'
            + "".join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' for i in range(1, len(sheets) + 1))
            + "</Types>",
        "_rels/.rels": '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            f'<Relationship Id="rId1" Type="{ns}/officeDocument" Target="xl/workbook.xml"/></Relationships>',
        "xl/workbook.xml": '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            f'<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{ns}"><sheets>'
            + "".join(f'<sheet name="{name}" sheetId="{i}" r:id="rId{i}"/>' for i, (name, _) in enumerate(sheets, start=1))
            + "</sheets></workbook>",
        "xl/_rels/workbook.xml.rels": '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            + "".join(f'<Relationship Id="rId{i}" Type="{ns}/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in range(1, len(sheets) + 1))
            + f'<Relationship Id="rId{len(sheets)+1}" Type="{ns}/styles" Target="styles.xml"/></Relationships>',
        "xl/styles.xml": '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
            '<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts>'
            '<fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>'
            '<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>'
            '<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>'
            '<cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>'
            '<xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/></cellXfs>'
            '</styleSheet>',
    }
    for i, (_, xml) in enumerate(sheets, start=1):
        files[f"xl/worksheets/sheet{i}.xml"] = xml
    out = Path(__file__).resolve().parent.parent / "templates" / "Asset_Import_Template.xlsx"
    out.parent.mkdir(exist_ok=True)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in files.items():
            z.writestr(name, data)
    print(out)


if __name__ == "__main__":
    main()
