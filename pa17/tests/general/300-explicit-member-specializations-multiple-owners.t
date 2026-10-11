// VALIDATION: compile-pass
// N3485 focus: 14.7.3 [temp.expl.spec]

template<class T>
struct owner {
  static int get();
};

struct X {};
struct Y {};

template<> int owner<X>::get() { return 1; }
template<> int owner<Y>::get() { return 2; }

int main()
{
  return owner<X>::get() + owner<Y>::get() == 3 ? 0 : 1;
}
