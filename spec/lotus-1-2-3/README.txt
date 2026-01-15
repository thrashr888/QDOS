Printing the README File

If you have completed the 1-2-3 Install program, you can use 1-2-3
to print the README file.  Start 1-2-3, select /File Retrieve, 
and then select file A_README.WK3.  Then select /Print Printer 
Align Go Page to print the A_README.WK3 file to your default printer 
using your default port.


If you have not completed the 1-2-3 Install program, you can use the
DOS COPY command to print the README file on your local parallel
printer.  To print the README file, enter the following command at
the DOS prompt in the 1-2-3 program directory:

copy readme.prn prn


INTRODUCTION

This README file contains information you may need to get up and
running with 1-2-3 for DOS Release 4, as well as information that will 
help you troubleshoot potential problems.  It includes the following 
sections:


  I.     "SUPPORT SERVICES" offers information on contacting Lotus 
       Technical Support and using both live and on-line support services.

  II.     "USABILITY AND PERFORMANCE TIPS" offers information on 
        optimizing usability and performance in 1-2-3.

  III.   "TROUBLESHOOTING" offers solutions for problems you may
      encounter while installing, starting, or using 1-2-3.

  IV.  "PRINTING INFORMATION" explains how a local print spooling utility 
      called BPrint can help you get the most out of your locally
     connected printer when you print using 1-2-3.

  V.  "PCL5 PRINTING INFORMATION" provides important information on
      the new PCL5 printer drivers included for devices
      such as the Hewlett-Packard Co. HP LaserJet 4 Series, LaserJet III Series,
      HP DeskJet 1200C, HP PaintJet XL300 and 100% compatible devices 
      such as the IBM 4039 LaserPrinter Series.



  I.   SUPPORT SERVICES

For information on using the automated support services Lotus offers, as 
well as information on support offerings and live support, see the topic 
"Lotus Support Services" in the main Help index.  To display the main Help 
index, press F1 when 1-2-3 is in READY mode.  The section "Lotus Support 
Services" also contains a list of frequently asked questions
and the answers to those questions.

  II. USABILITY AND PERFORMANCE TIPS

1-2-3 Release 4 for DOS comes with pre-defined named styles.  If you want to
use these named styles with worksheets created in previous releases of 1-2-3, 
select :Special Import Named-Styles, and specify the file STYLES.FM3, which
is in the 1-2-3 product directory.

The default row height in 1-2-3 Release 4 may be slightly smaller than in some
previous releases of 1-2-3.  As a result, in some worksheets the double-
underline format and the underscore character may not display on the screen.
To increase the height of a particular row, select :Worksheet Row. To increase
the default row height, select :Display Row.  Alternately, you can increase the
default row height by turning off the display of either worksheet tabs or
scroll bars.  To do so, select /Worksheet Global Default Other Display.

If you will be using Version Manager and you will be sharing the
WK3 file with others who have Release 3.x of 1-2-3, protect the file to prevent
users from deleting range names.  To protect a file, select /Worksheet Global
Prot Enable.  For more information on protecting files, refer to Help or
the User's Guide.

1-2-3 uses hard disk space for a number of operations, including file swapping
and printing.  To maximize the speed at which 1-2-3 can access your hard
drive, you should run a disk optimization program such as Defrag
on a regular basis.

For other performance tips, please refer to on-line Help.
Select "How Do I...? Index" from the main Help index and then select 
"Improve the Performance of 1-2-3."


  III.   TROUBLESHOOTING

This section describes some of the problems you may encounter while
installing, starting, or using 1-2-3 and lists possible solutions for
these problems.


  IIIA. Install Problems

Problem: You tried to start Install, and this error message appeared
on your screen: "There is not enough memory available to run Install.
Remove all memory-resident programs and start Install again."

Solution: You need a minimum of approximately 420 KB of available
conventional memory to use Install.  To determine how much
conventional memory is currently available, use the DOS CHKDSK
command and check the number of "bytes free." Then follow the
instructions in Chapter 4 of the User's Guide to
modify your CONFIG.SYS or AUTOEXEC.BAT files so that they do not
start (or allocate memory for) any unnecessary software programs when
you start your computer.  After you modify the files, reboot your
computer and try to start Install again.

Problem: You started Install, but cannot proceed past the first
Install screen.  (The system "hangs.")

Solution: This problem could arise for two reasons.  When you start
Install, the program searches for information about your hardware and
may "hang" if it encounters hardware it does not recognize.  To
disable the hardware detection process when you start Install, start
the program by entering the following command at the DOS prompt:

install -h

This problem can also occur when you have a
terminate-and-stay-resident program running.  Follow the instructions
in Chapter 4 of the User's Guide to modify your
CONFIG.SYS or AUTOEXEC.BAT files so that they do not start (or
allocate memory for) any unnecessary software programs when you start
your computer.  After you modify the files, reboot your computer and
try to start Install again.


  IIIB. Problems Starting 1-2-3

This section lists error messages that may appear when you try to
start 1-2-3 and explains how to remedy each startup problem.  The
messages are listed in alphabetical order.


Message: "Cannot create swap file..."

Solution: This message appears if the 123SWAPPATH statement in your
AUTOEXEC.BAT file refers to a directory that does not exist or to a
network drive that you cannot write to.  Edit the 123SWAPPATH
statement to refer to an existing directory or a directory to which
you can write information.


Message: "Cannot use extended memory: HIMEM.SYS not version 2"

Solution: Your CONFIG.SYS file contains a reference (in the form
DEVICE=HIMEM.SYS) to a HIMEM.SYS file that is out of date.  Either
replace the HIMEM.SYS file with an updated version or remove the device
statement from your CONFIG.SYS file.


Message: "Not enough disk space for swapping - required bytes:"

Solution: This message appears if you don't have enough space on your
hard disk to create a swap file.  Delete any unnecessary files,
beginning with the extra swap files 1-2-3 may have already created
(one for each failed attempt to start 1-2-3).  The swap files are in
the 1-2-3 program directory or, if your AUTOEXEC.BAT file includes a
123SWAPPATH statement, in the directory specified in that statement.
Swap file names have eight characters and no extension and look like
random sequences of letters and/or numbers.


Message: "Not enough memory to start 123"

Solution: The amount of memory you currently have available
(conventional plus extended) is not sufficient to start 1-2-3. 
Follow the instructions in Chapter 4 of the User's Guide 
to modify your CONFIG.SYS or AUTOEXEC.BAT files so that they do
not start (or allocate memory for) any unnecessary software programs
when you start your computer.


Message: "Requires DOS 3.0 or later"

Solution: To use 1-2-3 Release 4 for DOS, you need to have DOS 3.0 or
higher.


Message: "System software does not follow VCPI or DPMI
specifications."

Solution: You are using an expanded memory manager that does not
support the Virtual Control Program Interface (VCPI) or DOS Protected
Mode Interface (DPMI) protocol.  Either delete the reference to the
expanded memory manager in your CONFIG.SYS file or obtain an updated 
version of the memory manager software from your vendor. 


Message: "There is not enough memory available to run 1-2-3.  Remove
all memory-resident programs and start 1-2-3 again."

Solution: This message can appear because you have too many programs
in memory.  Follow the instructions in Chapter 4 of the User's Guide to 
modify your CONFIG.SYS or AUTOEXEC.BAT files so
that they do not start (or allocate memory for) any unnecessary
software programs when you start your computer.


Problem: In the few moments between the time you start 1-2-3 and the
worksheet appears, the 1-2-3 name and logo never appear; the screen
either remains blank or flickers.

Solution: 1-2-3 is attempting to display the 1-2-3 name and logo for
a graphics display, but you have a character-based display.  To
display the 1-2-3 name and logo for your character-based display,
start 1-2-3 as follows:

If you have a color monitor, make sure the DOS prompt appears on the
screen and then start 1-2-3 by typing 123 -c and pressing ENTER.

If you have a monochrome monitor, start 1-2-3 by typing 123 -m and
pressing ENTER.

Problem: You received this error message: "General Protection Fault"

Solution: This error may appear for a number of reasons.  The most
common reason is that a terminate-and-stay-resident program may not
be compatible with 1-2-3.  Follow the instructions in Chapter 4 of
the User's Guide to modify your CONFIG.SYS or
AUTOEXEC.BAT files so that they do not start (or allocate memory for)
any software program that is not absolutely necessary for the
functioning of your computer.  Then, restart your computer and 1-2-3.
To determine which of the CONFIG.SYS or AUTOEXEC.BAT statements you
removed was interfering with 1-2-3, restore the statements to the
file(s) one at a time and restart the computer and 1-2-3 each time.



  IV. PRINTING INFORMATION

  IVA.  BPrint - a Local Print Spooling Utility

1-2-3 includes an improved version of the BPrint utility originally 
included in Releases 2.3, 2.4, and 3.4.  BPrint is a TSR (terminate-and-
stay-resident) program that you load prior to starting 1-2-3.  It provides 
you with the option of printing in the background to a local printer (using 
BPrint) or in the foreground using the default 1-2-3 printing system.  

To use BPrint, you must load it before you start 1-2-3.  At a DOS prompt 
in the 1-2-3 program directory, type bprint and press ENTER.  Then start
1-2-3 and select either /Print Background or :Print Background.


  IVB.  When should I use BPrint?

Use BPrint under the following circumstances:

-When you are printing a large spreadsheet on a locally connected printer
-When immediate output is not required
-When you need to continue working in your spreadsheet while you print to 
 a locally connected printer.

Do NOT use BPrint under the following circumstances:

-When you are running under Microsoft Windows
-When you are printing across a network
-When you need to have the output immediately

  IVC.  How has BPrint been improved?

BPrint has been improved in three important ways.  First, 
printing with BPrint is now faster than it was in Release 3.4.  
Second, you can now remove BPrint from memory after you exit 1-2-3.  
(To do this, enter bprint -q at a DOS prompt.) Finally, you are no 
longer prompted to enter a filename for the temporary print file.  Your 
data is automatically sent to a print file on disk immediately after you 
choose /Print or :Print Background.

  IVD.  Additional BPrint information

To obtain on-line Help about the available BPrint parameters, enter
bprint -h at the DOS prompt in the 1-2-3 program directory, or review 
the on-line Help information on "Using BPrint."

BPrint can only send data to a direct serial or parallel connection, and not
to a networked printer.  However, you don't need to use BPrint to print to a 
networked printer, because network printers utilize spoolers provided by 
network software.  These spoolers provide the same capability as BPrint in
1-2-3.  It would be redundant and unnecessarily slow for 1-2-3 to spool a
print job out to BPrint, then have BPrint spool the job out to the network
spooler, and then have the network spooler send the job to the printer.
You will more quickly regain keyboard control and receive faster output
when you print directly to the network spooler.

  V.  PCL5 PRINTING INFORMATION

  VA.  General Information about PCL5 and /Print Drivers

1-2-3 includes new and improved drivers for printing on the
Hewlett-Packard LaserJet 4 series, the LaserJet III series,
the DeskJet 1200C, the PaintJet XL300, and 100% compatible
printers such as the IBM 4039 LaserPrinter series. 

For each of these devices, you can now specify two different types of 
drivers:  PCL5 drivers and standard printer drivers.  The new PCL5 printer
drivers provide enhanced print speed and functionality when you print
with the Wysiwyg command :Print.  Improvements in the standard
printer drivers let you print more quickly when you print with the
/Print command.  Both types of drivers print to both paper and
transparency media.

You can use PCL5 drivers to print with Wysiwyg using print
features such as scaleable font sizes and colors, as well as vector
graphics drawing. These features were previously available only
if you used the /Print menu to specify setup strings. The PCL5
drivers provide these features at the fastest print speed for Wysiwyg and 
make it unnecessary to use setup strings.

PCL5 PRINTER DRIVERS INCLUDED IN 1-2-3

    HP LaserJet III series (PCL5 WYSIWYG)
    HP LaserJet III Si (PCL5 WYSIWYG)
    HP LaserJet 4 series (PCL5 WYSIWYG)
       (supports models 4, 4Si, 4L, and 4P)
    HP PaintJet XL300 (PCL5 WYSIWYG)
    HP DeskJet 1200C (PCL5 WYSIWYG)
    IBM 4039 LaserPrinter (PCL5 WYSIWYG)
       (utilizes HP LaserJet III Si default emulation)

NOTE: IF YOU DO NOT SEE YOUR PRINTER NAME OR MANUFACTURER LISTED
when you use Install to select one or more printer drivers, you should 
choose the HP printer that is compatible with your
printer. For more information, see your printer documentation
or call your printer manufacturer.

NOTE: PCL5 DRIVERS ARE NOT RECOMMENDED FOR PRINTING WITH /PRINT.
Drivers with the description "(PCL5 WYSIWYG)" may produce
unexpected results and less than optimal print speed if you use
them when you print with /Print. However, you can use printer drivers 
with the description "(/Print)" to speed up printing with /Print. 

  VB.  What driver(s) should I install?

 Printer drivers with the description "(PCL5 WYSIWYG)"
      provide enhanced Wysiwyg (:Print) printing. For
      example, if you have an HP LaserJet 4 series printer
      and want enhanced Wysiwyg printing, you would select
      HP LJ 4 Series (PCL5 WYSIWYG). Note that
      you can also use PCL5 drivers to provide enhanced graphics 
      printing when you print with /Print [B,E,P] Image.

      Printer drivers with the description "(/Print)" provide the 
      fastest output speed when you print with /Print.  For example, if you
      have an HP LaserJet 4 series printer and want the fastest
      possible output speed when you print with /Print, you would select
      HP LJ 4 Series (/Print).

Lotus recommends you install both the "(PCL5 WYSIWYG)" and
"(/Print)" printer driver options that correspond to your
printer model to satisfy all your potential printing needs.

For more information about using Install to change your
printer driver selections, see Chapter 3 of the User's Guide.

  VC.  Using PCL5 drivers with 1-2-3 

The following fonts, available when Wysiwyg is in memory,
will automatically map to your HP or compatible printer:

Swiss   -> Univers
Dutch   -> CG Times
Courier -> Courier


PCL5 FEATURES SUPPORTED:

    On Board Scaleable Typeface Selection:
        Univers, CG Times, and Courier

    Symbol Sets:
        Wingdings (for LJ 4 series, DJ 1200C, and compatibles)
        Dingbats  (for LJ III Si series and compatibles)
        Roman8    (for LJ III series, PJ XL300, and compatibles)

    Additional Features Introduced on HP-GL/2:
        Full vector graphics drawing
        True text rotation

    PJL Support:
        PCL5 language switching

    Paper Handling:
        Two input bins

ACCESSING ADDITIONAL TYPEFACES ON HP LASERJET 4 SERIES AND 
DESKJET 1200C PRINTERS

You can select additional decorative scaleable typefaces on the HP LaserJet 4 
series, the DeskJet 1200C, or compatible printers by specifying them with 
:Format Font Replace (1 - 8) Other.  You can specify the following typefaces:

    Albertus           HPALBERT.IFL
    Antique Olive      HPANTOLV.IFL
    Coronet            HPCORNET.IFL
    Garamond Antiqua   HPGARAMD.IFL
    Letter Gothic      HP-GOTH.IFL
    Marigold           HPMARGLD.IFL
    CG Omega           HPOMEGA.IFL

Not all of these decorative fonts are available in bold,
italics, or bold-italics with your printer. For example,
Marigold is available in a normal typeface only, so if you
use :Format Bold or Italics, these attributes will not appear
in your print job. In addition, Coronet is an automatically
italicized font, so you do not have to select :Format Italics.

Where an attribute is not available, your printer will apply
the most appropriate fallback attributes to your print job
based upon your printer's capabilities. To determine which
font attributes are available for each typeface, perform a
print test of the fonts in your printer. For more information
on performing a print test, see your printer documentation.

For more information on using fonts in Wysiwyg,  
see Chapter 9 in the User's Guide.

IMPROVING WYSIWYG SCREEN DISPLAY MATCHING

Use the following steps to select your HP printer font by name 
in Wysiwyg so that the fonts displayed in Wysiwyg will better match 
the scaleable fonts your PCL5 driver uses when you print with Wysiwyg.


  1.  Select :Format Font Replace (1 - 8) Other.

NOTE: Remember that Font 1 is your default typeface. To change the 
typeface in a cell or range of cells, select :Format Font and the number 
from 1 to 8 that corresponds to your currently defined library
of eight typefaces.

  2.  Select the appropriate printer font (HPxxxxxx.IFL).

    Swiss   -> Univers  -> HP-UNVRS.IFL
    Dutch   -> CG Times -> HP-TIMES.IFL
    Courier -> Courier  -> HP-COUR.IFL

  3.  Specify a point size from 1 to 72 for the selected
    typeface.

  4.  Select /File Save to save your worksheet file. This
    guarantees that 1-2-3 will use the new .IFL files and
    that enhancements in the updated drivers will take effect.

  5.  If you want to apply your font selections to ALL new
    worksheet files as you create them, select :Format Font
    Default Update to save these font selections as your
    default font library.

    NOTE: If you want to use your font selections with selected
    worksheet files, you can store your font selections in named
    font libraries instead of in the default font library.
    Use :Format Font Library Save to create a font library.
    For more information, see Chapter 9 in the User's Guide.

If you want to use the fonts in your default font library with
an existing worksheet file, retrieve the existing file and
select :Format Font Default Restore to apply the fonts in your
default font library. To update the file, be sure to save
these changes using /File Save.

For more information, see Chapter 9 in the User's Guide.

Both the HP PaintJet XL300 PCL5 and DeskJet 1200C PCL5 drivers
support color printing using the Wysiwyg :Print menu. The new PCL5
drivers make it unnecessary to use additional print settings or setup 
strings to add color to your print jobs.

  VD.  Color notes for HP PaintJet XL300 and DeskJet 1200C Users

ADDING COLOR TO YOUR WORKSHEET DATA

Use :Format Color Text and Background to change the text and
background colors in a particular cell. If you use
:Format Color to add color to your worksheet data, remember
to save your work with /File Save.  Work that you print with
the Wysiwyg command :Print matches what you see on the screen as
closely as possible.

For more information on formatting with color, select
:Format Color and press F1 (HELP).

NOTE:  Certain Wysiwyg limitations can cause some colors
to be displayed differently on screen and in print.
Before you use the new PCL5 drivers to print in color,
you may want to print worksheet file COLORTST.WK3, which is
supplied with 1-2-3.  COLORTST.WK3 lets you compare
the colors you can print to the colors displayed
on the screen.
