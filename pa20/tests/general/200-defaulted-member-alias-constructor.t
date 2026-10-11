template<class D>
struct S {
  template<bool B = true>
  using Good = D&&;

  template<bool B = true, class = int>
  S(int, Good<B>) {}
};

void deleter(void*) {}

void f() {
  S<void (*)(void*)> s(0, deleter);
}

int main() {
  f();
  return 0;
}
