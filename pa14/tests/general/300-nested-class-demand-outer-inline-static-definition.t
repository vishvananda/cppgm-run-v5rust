template<class> struct c {
  static void f() {}
  struct n { static void g() { f(); } };
};
int main() { c<int>::n::g(); }
