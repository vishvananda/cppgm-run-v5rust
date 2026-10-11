template<class C, class Traits>
struct Stream {};

template<class C, class Traits>
Stream<C, Traits>& manip(Stream<C, Traits>& s) {
  return s;
}

struct Char {};
struct Traits {};

using MyStream = Stream<Char, Traits>;

void call(MyStream& (*f)(MyStream&)) {
  MyStream s;
  f(s);
}

int main() {
  call(manip);
  return 0;
}
