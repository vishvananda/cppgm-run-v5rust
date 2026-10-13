// Storage, linkage and declaration identity regression; frozen C++ behavior.
extern "C" {void f(){extern int x;}}extern "C++" int x;
