namespace ns {
template<class T> struct hash {};
template<class A, class B, class C> struct unordered_map {};
}

namespace alias = ns;
using namespace alias;

enum E { A };
typedef int token_op;

unordered_map<E, token_op, hash<int> > m = {
  {A, 0}
};
