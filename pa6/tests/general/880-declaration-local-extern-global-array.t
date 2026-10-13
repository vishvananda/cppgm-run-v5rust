// Storage, linkage and declaration identity regression; frozen C++ behavior.
extern int x[2];void f(){extern int x[];sizeof(x);}
