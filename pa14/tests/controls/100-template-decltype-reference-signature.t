// N3485 7.1.6.2p4, 14.7.1p1: substituted decltype preserves declared
// member types and lvalue/xvalue categories without demanding unused bodies; expect exit 0
struct Box { int value; };
template<class T> auto member(T& x) -> decltype((x.value)) { return x.value; }
template<class T> auto move_ref(T& x) -> decltype(static_cast<T&&>(x))
{ return static_cast<T&&>(x); }
template<class T> auto indirect(T p) -> decltype(p()) { return p(); }
template<class T> auto unused(T x) -> decltype(x + 1) { return x.missing(); }
int answer() { return 7; }
int main()
{
  Box box = {3};
  member(box) = 5;
  int n = 6;
  int&& moved = move_ref(n);
  moved = 8;
  return box.value == 5 && n == 8 && indirect(answer) == 7
    && sizeof(unused(0)) == sizeof(int) ? 0 : 1;
}
