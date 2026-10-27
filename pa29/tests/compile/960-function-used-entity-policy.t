__attribute__((used)) int f();
int f() { return 7; }
template<class T> __attribute__((used)) int g() { return sizeof(T); }
template<class T> struct X {
  __attribute__((used)) int h() { return sizeof(T); }
};
int main() {
  auto lambda = []() __attribute__((noinline, pure, used)) { return 1; };
  X<int> object;
  return f() + g<int>() + object.h() + lambda() - 16;
}
