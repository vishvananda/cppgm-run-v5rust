template <typename T>
void f(T t) {
  typedef __decltype(*t) Ref;
  typedef __decltype(*t) AltRef;
}

int main() {
  int x = 0;
  f(&x);
  return 0;
}
