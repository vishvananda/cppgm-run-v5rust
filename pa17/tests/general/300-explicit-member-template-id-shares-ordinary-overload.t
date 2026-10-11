// A non-template member and a member function template with the same name are
// one overload set. The ordinary overload must not make the explicit
// template-id parse as relational operators.
struct S {
  int choose(const char*);

  template<class U>
  int choose(int x) {
    return x;
  }

  int run() {
    return choose<int>(0);
  }
};

int S::choose(const char*) {
  return 7;
}

int main() {
  S s;
  return s.run();
}
