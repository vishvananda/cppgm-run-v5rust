// VALIDATION: compile-pass
// N3485 focus: 14.7.3 [temp.expl.spec]

template<class T>
struct owner {
  template<int I> static int get();
};

struct X {};
struct Y {};

template<> template<>
int owner<X>::get<0>() { return 1; }

template<> template<>
int owner<X>::get<1>() { return 2; }

template<> template<>
int owner<Y>::get<0>() { return 4; }

int main() {
  return owner<X>::get<0>() + owner<Y>::get<0>() == 5 ? 0 : 1;
}
