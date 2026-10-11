namespace lib {

template <class T>
struct WrapIter {
  T p;
};

template <class T>
bool operator==(const WrapIter<T> &, const WrapIter<T> &) {
  return true;
}

template <class T, class U>
bool operator==(const WrapIter<T> &, const WrapIter<U> &) {
  return false;
}

}  // namespace lib

namespace outer {

int f() {
  struct Local {};
  lib::WrapIter<Local *> a;
  lib::WrapIter<Local *> b;
  return a == b ? 0 : 1;
}

}

int main() {
  return outer::f();
}
