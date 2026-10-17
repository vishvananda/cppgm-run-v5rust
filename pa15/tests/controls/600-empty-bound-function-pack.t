// N3485 14.8.2p8: an empty deduction cannot expand against a nonempty bound pack.
// expect exit 0
template<class T, class U> struct Pair {};
template<class... T> struct Owner
{
  int call(...) { return 7; }
  template<class... U> int call(Pair<T, U>...) { return sizeof...(U); }
  template<class... U> int only(Pair<T, U>...) { return sizeof...(U); }
  template<class... U> int explicit_call(Pair<T, U>...) { return 3; }
  template<class... U> int explicit_call(...) { return 4; }
};
int main()
{
  Owner<int> one;
  Owner<> empty;
  return one.call() != 7 || empty.only() != 0 ||
         one.call(Pair<int, char>()) != 1 || one.explicit_call<char, long>() != 4;
}
