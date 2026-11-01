/*
 * filter.c - the MSJ filter extension (Notenboom, MSJ Sep 1988, Fig. 3), ported to modern C
 *
 * Replaces its argument with the argument run through a filter program, for
 * example "sort <". Changes from the 1988 listing: standard prototypes, no
 * pascal/far, an own max macro, and rm instead of DEL outside Windows.
 *
 *   NOARG   - filter the current line
 *   NULLARG - filter from the cursor to the end of the line
 *   LINEARG - filter a range of lines (also NUMARG and MARKARG)
 *   BOXARG  - filter the selected box
 *   TEXTARG - set the filter command
 */
#include <string.h>
#include "ext.h"

#define ID " filter ver 1.00"
#define max(a, b) ((a) > (b) ? (a) : (b))

#ifdef _WIN32
#define REMOVE "del "
#else
#define REMOVE "rm "
#endif

static flagType DoSpawn(char *);
static void id(char *);
static int SetFilter(char *);

static PFILE pFileFilt = 0;              /* handle for filter-file */
static char *szNameFilt = "<filter-file>";
static char *szTemp1 = "filter1.tmp";
static char *szTemp2 = "filter2.tmp";
static char filtcmd[BUFLEN] = "";        /* filter command itself */

flagType filter(unsigned int argData, ARG *pArg, flagType fMeta)
{
    char buf[BUFLEN];
    int cbLineMax;
    LINE cLines;
    LINE iLineCur;
    PFILE pFile;

    (void)argData;
    (void)fMeta;
    id("");
    pFile = FileNameToHandle("", "");
    DelFile(pFileFilt);

    /* Step 1: copy the selected region into the filter-file. */
    switch (pArg->argType) {
    case NOARG:
        CopyLine(pFile, pFileFilt, pArg->arg.noarg.y, pArg->arg.noarg.y, 0);
        break;
    case NULLARG:
        CopyStream(pFile, pFileFilt, pArg->arg.nullarg.x, pArg->arg.nullarg.y, 255, pArg->arg.nullarg.y, 0, 0);
        break;
    case LINEARG:
        CopyLine(pFile, pFileFilt, pArg->arg.linearg.yStart, pArg->arg.linearg.yEnd, 0);
        break;
    case BOXARG:
        CopyBox(pFile, pFileFilt, pArg->arg.boxarg.xLeft, pArg->arg.boxarg.yTop, pArg->arg.boxarg.xRight, pArg->arg.boxarg.yBottom, 0, 0);
        break;
    case TEXTARG:
        SetFilter(pArg->arg.textarg.pText);
        return 1;
    }

    /* Step 2: write the selected text to disk. */
    if (!FileWrite(szTemp1, pFileFilt)) {
        id("** Error writing temporary file **");
        return 0;
    }

    /* Step 3: filter command + " " + temp 1 + " >" + temp 2. */
    strcpy(buf, filtcmd);
    strcat(buf, " ");
    strcat(buf, szTemp1);
    strcat(buf, " >");
    strcat(buf, szTemp2);
    if (!DoSpawn(buf)) {
        id("** Error executing filter **");
        return 0;
    }

    /* Step 4: replace the filter-file with the second temp file. */
    DelFile(pFileFilt);
    if (!FileRead(szTemp2, pFileFilt)) {
        id("** Error reading temporary file **");
        return 0;
    }

    /* Step 5: replace the region with the filtered text. */
    cLines = FileLength(pFileFilt);
    cbLineMax = 0;
    for (iLineCur = 0; iLineCur < cLines; iLineCur++)
        cbLineMax = max(cbLineMax, GetLine(iLineCur, buf, pFileFilt));

    switch (pArg->argType) {
    case NOARG:
        DelLine(pFile, pArg->arg.noarg.y, pArg->arg.noarg.y);
        CopyLine(pFileFilt, pFile, 0, 0, pArg->arg.noarg.y);
        break;
    case NULLARG:
        DelStream(pFile, pArg->arg.nullarg.x, pArg->arg.nullarg.y, 255, pArg->arg.nullarg.y);
        CopyStream(pFileFilt, pFile, 0, 0, cbLineMax, 0, pArg->arg.nullarg.x, pArg->arg.nullarg.y);
        break;
    case LINEARG:
        DelLine(pFile, pArg->arg.linearg.yStart, pArg->arg.linearg.yEnd);
        CopyLine(pFileFilt, pFile, 0, cLines - 1, pArg->arg.linearg.yStart);
        break;
    case BOXARG:
        DelBox(pFile, pArg->arg.boxarg.xLeft, pArg->arg.boxarg.yTop, pArg->arg.boxarg.xRight, pArg->arg.boxarg.yBottom);
        CopyBox(pFileFilt, pFile, 0, 0, cbLineMax - 1, cLines - 1, pArg->arg.boxarg.xLeft, pArg->arg.boxarg.yTop);
        break;
    }

    /* Clean up the temporary files. */
    strcpy(buf, REMOVE);
    strcat(buf, szTemp1);
    DoSpawn(buf);
    strcpy(buf + strlen(REMOVE), szTemp2);
    DoSpawn(buf);
    return 1;
}

/* Runs an operating system command through the editor's shell function. */
static flagType DoSpawn(char *szCmd)
{
    static char cmd[BUFLEN];

    strcpy(cmd, "arg \"");
    strcat(cmd, szCmd);
    strcat(cmd, "\" shell");
    return fExecute(cmd);
}

/* Saves the filter command: the filtcmd switch or a text argument. */
static int SetFilter(char *szCmd)
{
    strcpy(filtcmd, szCmd);
    return 1;
}

/* Creates <filter-file> and assigns Alt+F. */
void WhenLoaded(void)
{
    pFileFilt = FileNameToHandle(szNameFilt, szNameFilt);
    if (!pFileFilt) {
        pFileFilt = AddFile(szNameFilt);
        FileRead(szNameFilt, pFileFilt);
    }
    SetKey("filter", "alt+f");
    id("text filter extension:");
}

/* Shows a message followed by the extension's name. */
static void id(char *pszFcn)
{
    static char buf[BUFLEN];

    strcpy(buf, pszFcn);
    strcat(buf, ID);
    DoMessage(buf);
}

struct swiDesc swiTable[] = {
    { "filtcmd", (void *)SetFilter, SWI_SPECIAL },
    { 0, 0, 0 }
};

struct cmdDesc cmdTable[] = {
    { "filter", filter, 0, KEEPMETA | NOARG | BOXARG | NULLARG | LINEARG | MARKARG | NUMARG | TEXTARG | MODIFIES },
    { 0, 0, 0, 0 }
};
