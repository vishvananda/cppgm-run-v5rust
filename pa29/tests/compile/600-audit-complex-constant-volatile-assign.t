typedef _Complex int C;
constexpr int check() {
 volatile C value={2,3};
 value=C{4,5};
 return 0;
}
static_assert(check()==0,"volatile complex assignment");
