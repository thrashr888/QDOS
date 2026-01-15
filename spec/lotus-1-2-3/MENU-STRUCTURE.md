# Lotus 1-2-3 Release 4 Menu Structure

Extracted from Lotus 1-2-3 4.0 for DOS (1994)

## Main Menu Categories (/ key)

The main menu is activated by pressing `/` (slash).

### Top-Level Commands

| Key | Menu | Description |
|-----|------|-------------|
| W | Worksheet | Worksheet-level settings and operations |
| R | Range | Operations on cell ranges |
| C | Copy | Copy cells |
| M | Move | Move cells |
| F | File | File operations |
| P | Print | Printing options |
| G | Graph | Chart/graph creation |
| D | Data | Database and data analysis |
| T | Tools | Add-ins and utilities |
| Q | Quit | Exit program |

## File Menu (/File)

| Command | Description |
|---------|-------------|
| Retrieve | Open a worksheet file |
| Save | Save current worksheet |
| Combine | Combine data from another file |
| Xtract | Extract portion to new file |
| Erase | Delete a file |
| List | List files |
| Import | Import from other formats |
| Dir | Change directory |
| Admin | Administrative functions |

### File Admin Submenu

| Command | Description |
|---------|-------------|
| Reservation | File reservation/locking |
| Seal | Seal file |
| Table-Link | Table linking |

## Worksheet Menu (/Worksheet)

| Command | Description |
|---------|-------------|
| Global | Global worksheet settings |
| Insert | Insert rows/columns |
| Delete | Delete rows/columns |
| Column | Column width settings |
| Erase | Erase ranges |
| Titles | Freeze titles |
| Window | Split window |
| Status | Show status |
| Page | Page breaks |
| Hide | Hide columns |

### Worksheet Global Submenu

| Command | Description |
|---------|-------------|
| Format | Default number format |
| Label | Default label alignment |
| Col-Width | Default column width |
| Recalc | Recalculation settings |
| Protection | Protection settings |
| Default | Save/restore defaults |
| Zero | Zero display setting |

## Range Menu (/Range)

| Command | Description |
|---------|-------------|
| Format | Cell format |
| Label | Label prefix |
| Erase | Erase range |
| Name | Named ranges |
| Justify | Justify text |
| Prot | Protect range |
| Unprot | Unprotect range |
| Input | Input range |
| Value | Convert formulas to values |
| Trans | Transpose range |

### Range Name Submenu

| Command | Description |
|---------|-------------|
| Create | Create named range |
| Delete | Delete named range |
| Labels | Create names from labels |
| Reset | Reset all names |
| Table | Show name table |
| Notation | Range notation style |

## Data Menu (/Data)

| Command | Description |
|---------|-------------|
| Fill | Fill range with series |
| Table | What-if tables (1/2/3-way) |
| Sort | Sort data |
| Query | Database query |
| Distribution | Frequency distribution |
| Matrix | Matrix operations |
| Regression | Statistical regression |
| Parse | Parse text strings |
| External | External database access |

### Data Query Submenu

| Command | Description |
|---------|-------------|
| Input | Define input range |
| Criteria | Define criteria range |
| Output | Define output range |
| Find | Find matching records |
| Extract | Extract matching records |
| Unique | Extract unique records |
| Delete | Delete matching records |
| Modify | Modify matching records |
| Reset | Reset query settings |
| Quit | Exit query menu |

## Print Menu (/Print)

| Command | Description |
|---------|-------------|
| Printer | Print to printer |
| File | Print to file |
| Encoded | Print encoded |
| Background | Background printing |

### Print Printer Submenu

| Command | Description |
|---------|-------------|
| Range | Print range |
| Line | Line advance |
| Page | Page advance |
| Options | Print options |
| Clear | Clear settings |
| Align | Align paper |
| Go | Start printing |
| Quit | Exit print menu |

## Graph Menu (/Graph)

| Command | Description |
|---------|-------------|
| Type | Graph type (Line/Bar/XY/Pie/etc.) |
| X | X-axis data range |
| A-F | Data ranges A through F |
| Reset | Reset graph |
| View | View graph |
| Save | Save graph |
| Options | Graph options |
| Name | Named graphs |
| Settings | Graph settings |
| Quit | Exit graph menu |

### Graph Type Options

| Type | Description |
|------|-------------|
| Line | Line graph |
| Bar | Vertical bar |
| XY | Scatter plot |
| Stacked-Bar | Stacked bars |
| Pie | Pie chart |
| HLCO | High-Low-Close-Open |
| Mixed | Mixed types |
| Area | Area chart |
| Radar | Radar/spider chart |
| 3D | 3D variations |

## Tools Menu (/Tools)

| Command | Description |
|---------|-------------|
| Analyze | Analysis tools |
| SmartIcons | SmartIcon bar |
| Macro | Macro operations |
| Config-Addins | Configure add-ins |

### Tools Analyze Submenu

| Command | Description |
|---------|-------------|
| Auditor | Formula auditor |
| Backsolver | Goal seeking |
| Solver | Optimization solver |

## Special/Colon Menu (:)

The colon `:` key accesses WYSIWYG formatting commands:

| Command | Description |
|---------|-------------|
| Format | WYSIWYG formatting |
| Print | WYSIWYG printing |
| Display | Display settings |
| Special | Special operations |

### :Format Submenu

| Command | Description |
|---------|-------------|
| Font | Font selection |
| Bold | Bold text |
| Italics | Italic text |
| Underline | Underline text |
| Color | Text/background color |
| Lines | Border lines |
| Shade | Cell shading |

## Key Bindings

| Key | Function |
|-----|----------|
| / | Main menu |
| : | WYSIWYG menu |
| F1 | Help |
| F2 | Edit cell |
| F3 | Name list |
| F4 | Absolute reference |
| F5 | Goto |
| F6 | Window switch |
| F7 | Query |
| F8 | Table |
| F9 | Calculate |
| F10 | Graph |
| Esc | Cancel/Back |
| Enter | Confirm |
| Tab | Next cell |

## @ Functions

Common spreadsheet functions (prefix with @):

### Math
- @SUM(range) - Sum of range
- @AVG(range) - Average
- @COUNT(range) - Count non-empty
- @MIN(range) - Minimum
- @MAX(range) - Maximum
- @ABS(value) - Absolute value
- @INT(value) - Integer part
- @ROUND(value,places) - Round
- @SQRT(value) - Square root

### Logic
- @IF(cond,true,false) - Conditional
- @TRUE - True value (1)
- @FALSE - False value (0)
- @AND(a,b,...) - Logical AND
- @OR(a,b,...) - Logical OR
- @NOT(value) - Logical NOT

### Text
- @LEFT(text,n) - Left characters
- @RIGHT(text,n) - Right characters
- @MID(text,start,n) - Middle characters
- @LEN(text) - Length
- @TRIM(text) - Remove spaces
- @UPPER(text) - Uppercase
- @LOWER(text) - Lowercase

### Date/Time
- @DATE(y,m,d) - Create date
- @NOW - Current date/time
- @TODAY - Current date
- @YEAR(date) - Extract year
- @MONTH(date) - Extract month
- @DAY(date) - Extract day

### Lookup
- @VLOOKUP(key,range,col) - Vertical lookup
- @HLOOKUP(key,range,row) - Horizontal lookup
- @INDEX(range,col,row) - Index into range

## Status Indicators

| Indicator | Meaning |
|-----------|---------|
| READY | Ready for input |
| MENU | Menu active |
| EDIT | Editing cell |
| VALUE | Entering value |
| LABEL | Entering label |
| POINT | Pointing to range |
| HELP | Help active |
| ERROR | Error condition |
| WAIT | Processing |
| CALC | Calculating |
| CIRC | Circular reference |
| OVR | Overwrite mode |
| END | End mode |
| CAPS | Caps lock |
| NUM | Num lock |
| SCROLL | Scroll lock |
