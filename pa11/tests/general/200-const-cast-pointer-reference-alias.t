struct S {
  using value_type = int*;

  static value_type pass(const value_type& p) {
    value_type& q = const_cast<value_type&>(p);
    return q;
  }
};

int main() {
  int x = 0;
  int* p = &x;
  return S::pass(p) == &x ? 0 : 1;
}
