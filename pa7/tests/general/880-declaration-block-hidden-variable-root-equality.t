// Storage, linkage and declaration identity regression; frozen C++ behavior.
static int x;void f(){int x;{extern int x;constexpr bool b=&x==&::x;int a[b?1:2];}}
