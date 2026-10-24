typedef _Complex int C;
constexpr int check() {
 volatile C value={2,3};
 C copy=value;
 return __real__ copy;
}
static_assert(check()==2,"volatile complex read");
