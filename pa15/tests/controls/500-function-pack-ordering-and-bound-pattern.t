// N3485 14.8.2.4p3, 14.8.2.5p8: trailing ordering and bound lockstep types;
// expect exit 0
template<class... T> int order(T...) { return 1; }
template<class T, class... U> int order(T, U...) { return 2; }
template<class T, class U> struct Pair {};
template<class T> int unused(T) { return 1; }
template<class T, class... U> int unused(T, U...) { return 2; }
template<class... T> struct Owner
{
  template<class... U> int call(Pair<T, U>...) { return sizeof...(U); }
};
int main()
{
  Owner<int, long> o;
  return unused(1) != 1 || order() != 1 || order(1, 2) != 2 ||
         o.call(Pair<int, char>(), Pair<long, short>()) != 2;
}
