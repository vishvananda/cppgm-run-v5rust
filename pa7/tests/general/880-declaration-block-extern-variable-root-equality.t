// Storage, linkage and declaration identity regression; frozen C++ behavior.
int x;void f(){extern int x;constexpr bool b=&x==&::x;int a[b?1:2];}
