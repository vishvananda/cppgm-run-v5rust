// Storage, linkage and declaration identity regression; frozen C++ behavior.
extern "C" int x;extern "C" int x;int f(){return x;}
