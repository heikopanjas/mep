/*
 * exthdr.c - forwards the low-level editor functions of ext.h to MEP
 *
 * The 1988 editor linked EXTHDR.OBJ into every extension for the same job
 * (M guide p. 69). Compile this file together with the extension.
 */
#include "ext.h"

EXPORT const struct MepCallbacks *MepCallbacks = 0;

PFILE    AddFile(char *name) { return MepCallbacks->AddFile(name); }
flagType BadArg(void) { return MepCallbacks->BadArg(); }
void     CopyBox(PFILE src, PFILE dst, COL xLeft, LINE yTop, COL xRight, LINE yBottom, COL xDst, LINE yDst) { MepCallbacks->CopyBox(src, dst, xLeft, yTop, xRight, yBottom, xDst, yDst); }
void     CopyLine(PFILE src, PFILE dst, LINE yStart, LINE yEnd, LINE yDst) { MepCallbacks->CopyLine(src, dst, yStart, yEnd, yDst); }
void     CopyStream(PFILE src, PFILE dst, COL xStart, LINE yStart, COL xEnd, LINE yEnd, COL xDst, LINE yDst) { MepCallbacks->CopyStream(src, dst, xStart, yStart, xEnd, yEnd, xDst, yDst); }
void     DelBox(PFILE file, COL xLeft, LINE yTop, COL xRight, LINE yBottom) { MepCallbacks->DelBox(file, xLeft, yTop, xRight, yBottom); }
void     DelFile(PFILE file) { MepCallbacks->DelFile(file); }
void     DelLine(PFILE file, LINE yStart, LINE yEnd) { MepCallbacks->DelLine(file, yStart, yEnd); }
void     DelStream(PFILE file, COL xStart, LINE yStart, COL xEnd, LINE yEnd) { MepCallbacks->DelStream(file, xStart, yStart, xEnd, yEnd); }
int      DoMessage(char *text) { return MepCallbacks->DoMessage(text); }
void     Display(void) { MepCallbacks->Display(); }
flagType fExecute(char *macro) { return MepCallbacks->fExecute(macro); }
LINE     FileLength(PFILE file) { return MepCallbacks->FileLength(file); }
PFILE    FileNameToHandle(char *name, char *shortName) { return MepCallbacks->FileNameToHandle(name, shortName); }
flagType FileRead(char *name, PFILE file) { return MepCallbacks->FileRead(name, file); }
flagType FileWrite(char *name, PFILE file) { return MepCallbacks->FileWrite(name, file); }
void     GetCursor(COL *x, LINE *y) { MepCallbacks->GetCursor(x, y); }
int      GetLine(LINE line, char *buf, PFILE file) { return MepCallbacks->GetLine(line, buf, file); }
void     KbHook(void) { MepCallbacks->KbHook(); }
void     KbUnHook(void) { MepCallbacks->KbUnHook(); }
void     MoveCur(COL x, LINE y) { MepCallbacks->MoveCur(x, y); }
void     pFileToTop(PFILE file) { MepCallbacks->pFileToTop(file); }
void     PutLine(LINE line, char *buf, PFILE file) { MepCallbacks->PutLine(line, buf, file); }
void     RemoveFile(PFILE file) { MepCallbacks->RemoveFile(file); }
int      ReadChar(void) { return MepCallbacks->ReadChar(); }
char    *ReadCmd(void) { return MepCallbacks->ReadCmd(); }
flagType Replace(char c, COL x, LINE y, PFILE file, flagType fInsert) { return MepCallbacks->Replace(c, x, y, file, fInsert); }
flagType SetKey(char *name, char *key) { return MepCallbacks->SetKey(name, key); }
