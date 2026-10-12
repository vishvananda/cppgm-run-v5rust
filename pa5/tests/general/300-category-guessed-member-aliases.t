template<class T> struct P {
  typedef T unknown;
  struct member { typedef int word; };
};
template<> struct P<int> { struct member { int word; }; };
using alias = P<int>::member;
typedef alias alias_again;
template<class T> using alias_template = typename P<T>::member;

void syntax() {
  P<int>::member::word * direct;
  alias::word * indirect;
  alias_again::word * typedef_chain;
  alias_template<int>::word * unknown_class;
}
