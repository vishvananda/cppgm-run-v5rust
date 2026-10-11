namespace lib {

template<class T>
struct Alloc {};

template<class T, class A = Alloc<T> >
struct Vec {};

} // namespace lib

struct Info {};

template<class P>
void push(const Info &, lib::Vec<P> &) {}

template<class P>
bool walk(P root) {
  lib::Vec<P> stack;
  push(*root, stack);
  return true;
}

int main() {
  Info info;
  return walk(&info) ? 0 : 1;
}
