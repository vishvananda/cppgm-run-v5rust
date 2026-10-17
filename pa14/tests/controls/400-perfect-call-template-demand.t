// N3485 13.3.3p1: identity non-template matches make template deduction moot.
// expect exit 0
template<class T> struct Trap
{
  static_assert(sizeof(T) == 0, "unneeded signature instantiation");
  using type = int;
};
int choose(int n) { return n; }
template<class T> typename Trap<T>::type choose(T);
int reference(int& n) { return n; }
template<class T> typename Trap<T>::type reference(T&);
int list(int) { return 1; }
template<class T> int list(T) { return 2; }
int main()
{
  int n = 3;
  return choose(2) != 2 || reference(n) != 3 || list({1}) != 1;
}
