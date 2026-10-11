template<class T>
struct S {
  S();
};

template<>
__attribute__((visibility("default"))) S<int>::S();

int main() {
  return 0;
}
