template<class T>
struct S {
  void f(const char*);
};

template<>
void S<int>::f(const char*);

int main() {
  return 0;
}
