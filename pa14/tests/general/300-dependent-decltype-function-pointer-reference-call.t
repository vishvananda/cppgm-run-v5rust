template<class T>
T&& declval();

struct Nat {};
Nat f(...);

template<class F>
decltype(declval<F>()()) f(F&&);

int main() {
  using Fn = int (*)();
  typedef decltype(f(declval<Fn&>())) R;
  R x = 0;
  return x;
}
