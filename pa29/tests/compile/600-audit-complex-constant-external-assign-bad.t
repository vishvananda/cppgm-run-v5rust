typedef _Complex int C;
C value={2,3};
constexpr int check() { value=C{4,5}; return 0; }
static_assert(check()==0,"external complex assignment");
