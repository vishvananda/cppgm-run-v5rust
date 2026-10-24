// PA29 designated initialization rejection.
struct S { int& a; int b; }; S value = {.b=2};
