struct S {
  template <bool U>
  void g(int);

  template <bool U>
  void f(int n, int bc);
};

template <bool U>
void S::f(int n, int bc) {
  if (n < bc)
    g<U>(n);
}

int main() { return 0; }
