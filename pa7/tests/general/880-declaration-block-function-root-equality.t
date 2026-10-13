// Storage, linkage and declaration identity regression; frozen C++ behavior.
int h(int);void f(){int h(int);constexpr bool b=h==::h;int a[b?1:2];}
