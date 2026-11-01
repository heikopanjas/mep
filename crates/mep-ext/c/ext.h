/*
 * ext.h - interface between MEP and C extensions (docs/spec/extensions.md)
 *
 * Declares the argument structure, the tables an extension exports and the
 * low-level editor functions. Compile exthdr.c together with the extension;
 * it forwards the functions to the editor.
 */
#ifndef MEP_EXT_H
#define MEP_EXT_H

#include <stdint.h>

#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif

/* The 1988 sources mark functions with these; they mean nothing today. */
#define pascal
#define far
#define EXTERNAL

#define BUFLEN 512

typedef int flagType;
typedef int32_t LINE;
typedef int32_t COL;
typedef struct fileType *PFILE;

/* Argument types and function flags (cmdDesc.argType) */
#define NOARG      0x00001
#define NULLARG    0x00002
#define TEXTARG    0x00004
#define NUMARG     0x00008
#define LINEARG    0x00010
#define STREAMARG  0x00020
#define BOXARG     0x00040
#define BOXSTR     0x00080
#define NULLEOL    0x00100
#define NULLEOW    0x00200
#define KEEPMETA   0x00400
#define CURSORFUNC 0x00800
#define WINDOWFUNC 0x01000
#define MODIFIES   0x02000
#define MARKARG    0x20000

struct noargType     { LINE y; COL x; };
struct textargType   { int cArg; LINE y; COL x; char *pText; };
struct nullargType   { int cArg; LINE y; COL x; };
struct lineargType   { int cArg; LINE yStart; LINE yEnd; };
struct streamargType { int cArg; COL xStart; LINE yStart; COL xEnd; LINE yEnd; };
struct boxargType    { int cArg; COL xLeft; LINE yTop; COL xRight; LINE yBottom; };

struct argType {
    int argType;
    union {
        struct noargType     noarg;
        struct textargType   textarg;
        struct nullargType   nullarg;
        struct lineargType   linearg;
        struct streamargType streamarg;
        struct boxargType    boxarg;
    } arg;
};
typedef struct argType ARG;

typedef flagType (*PCMD)(unsigned int argData, ARG *pArg, flagType fMeta);
typedef int (*PIF)(char *);

struct cmdDesc {
    char *name;
    PCMD func;
    void *reserved;
    unsigned int argType;
};

#define SWI_BOOLEAN 0
#define SWI_NUMERIC 1
#define SWI_SPECIAL 2

struct swiDesc {
    char *name;
    void *act;   /* int * for SWI_BOOLEAN and SWI_NUMERIC, PIF for SWI_SPECIAL */
    int type;
};

/* Required exports (p. 70) */
EXPORT extern struct cmdDesc cmdTable[];
EXPORT extern struct swiDesc swiTable[];
EXPORT void WhenLoaded(void);

/* Low-level editor functions (MSJ Fig. 7) */
PFILE    AddFile(char *name);
flagType BadArg(void);
void     CopyBox(PFILE src, PFILE dst, COL xLeft, LINE yTop, COL xRight, LINE yBottom, COL xDst, LINE yDst);
void     CopyLine(PFILE src, PFILE dst, LINE yStart, LINE yEnd, LINE yDst);
void     CopyStream(PFILE src, PFILE dst, COL xStart, LINE yStart, COL xEnd, LINE yEnd, COL xDst, LINE yDst);
void     DelBox(PFILE file, COL xLeft, LINE yTop, COL xRight, LINE yBottom);
void     DelFile(PFILE file);
void     DelLine(PFILE file, LINE yStart, LINE yEnd);
void     DelStream(PFILE file, COL xStart, LINE yStart, COL xEnd, LINE yEnd);
int      DoMessage(char *text);
void     Display(void);
flagType fExecute(char *macro);
LINE     FileLength(PFILE file);
PFILE    FileNameToHandle(char *name, char *shortName);
flagType FileRead(char *name, PFILE file);
flagType FileWrite(char *name, PFILE file);
void     GetCursor(COL *x, LINE *y);
int      GetLine(LINE line, char *buf, PFILE file);
void     KbHook(void);
void     KbUnHook(void);
void     MoveCur(COL x, LINE y);
void     pFileToTop(PFILE file);
void     PutLine(LINE line, char *buf, PFILE file);
void     RemoveFile(PFILE file);
int      ReadChar(void);
char    *ReadCmd(void);
flagType Replace(char c, COL x, LINE y, PFILE file, flagType fInsert);
flagType SetKey(char *name, char *key);

/* The table exthdr.c forwards through; the editor fills MepCallbacks when it loads the library */
struct MepCallbacks {
    int      version;
    PFILE    (*AddFile)(char *);
    flagType (*BadArg)(void);
    void     (*CopyBox)(PFILE, PFILE, COL, LINE, COL, LINE, COL, LINE);
    void     (*CopyLine)(PFILE, PFILE, LINE, LINE, LINE);
    void     (*CopyStream)(PFILE, PFILE, COL, LINE, COL, LINE, COL, LINE);
    void     (*DelBox)(PFILE, COL, LINE, COL, LINE);
    void     (*DelFile)(PFILE);
    void     (*DelLine)(PFILE, LINE, LINE);
    void     (*DelStream)(PFILE, COL, LINE, COL, LINE);
    int      (*DoMessage)(char *);
    void     (*Display)(void);
    flagType (*fExecute)(char *);
    LINE     (*FileLength)(PFILE);
    PFILE    (*FileNameToHandle)(char *, char *);
    flagType (*FileRead)(char *, PFILE);
    flagType (*FileWrite)(char *, PFILE);
    void     (*GetCursor)(COL *, LINE *);
    int      (*GetLine)(LINE, char *, PFILE);
    void     (*KbHook)(void);
    void     (*KbUnHook)(void);
    void     (*MoveCur)(COL, LINE);
    void     (*pFileToTop)(PFILE);
    void     (*PutLine)(LINE, char *, PFILE);
    void     (*RemoveFile)(PFILE);
    int      (*ReadChar)(void);
    char    *(*ReadCmd)(void);
    flagType (*Replace)(char, COL, LINE, PFILE, flagType);
    flagType (*SetKey)(char *, char *);
};

#define MEP_CALLBACKS_VERSION 1

EXPORT extern const struct MepCallbacks *MepCallbacks;

#endif
