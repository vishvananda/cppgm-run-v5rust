// Storage, linkage and declaration identity regression; frozen C++ behavior.
extern "C" {int h(int);extern "C++" {int k(int);}int g(int);}int h(int);int k(int);int g(int);
