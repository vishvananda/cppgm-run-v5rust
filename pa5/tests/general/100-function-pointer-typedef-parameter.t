// HHC-043: intended outcome: should parse successfully as a real function declaration.
typedef long fpos_t;
typedef int FILE;
FILE *funopen(const void *, fpos_t (*)(void *, fpos_t, int));
