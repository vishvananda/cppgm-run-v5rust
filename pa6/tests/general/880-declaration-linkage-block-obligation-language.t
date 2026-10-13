// Storage, linkage and declaration identity regression; frozen C++ behavior.
extern "C" {void f(){int h(int);}}extern "C++" int h(int);
