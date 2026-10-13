// Storage, linkage and declaration identity regression; frozen C++ behavior.
const int x=3;void f(){extern const int x;constexpr int n=x;}
