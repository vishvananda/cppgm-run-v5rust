// N3485 7.1.6.4p2-p3: auto with a function declarator needs a
// trailing-return-type; `auto (*p)() = 0` is ill-formed. Must reject.
int main() { auto (*p)() = 0; return p == 0 ? 0 : 1; }
