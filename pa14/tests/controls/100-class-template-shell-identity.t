// expect exit 0
// Class identity, deduction and pointer calls require no complete specialization.
template<class T, class U = T*> struct Box;
using IntBox = Box<int>;
using IntBox = Box<int, int*>;
template<class U, class V> struct Box;

template<class T, class U> int measure(Box<T, U>*) {
  return sizeof(T) + sizeof(U);
}
template<class T> Box<T>* empty(T*) { return 0; }
namespace N {
  template<class T> struct Other;
  int probe(const Other<int>*) { return 7; }
}
int main() {
  return measure((IntBox*)0) == 12 && empty((char*)0) == 0 &&
         N::probe((N::Other<int>*)0) == 7 ? 0 : 1;
}
