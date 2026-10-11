template<class T>
struct S {
  using value_type = T;
  static void assign(value_type&, const value_type&);
};

S<char>* before = 0;

template<>
struct S<char> {
  using value_type = char;
  static inline void assign(value_type& a, const value_type& b) { a = b; }
};

int f() {
  char x = 0;
  S<char>::assign(x, 'a');
  return x;
}

int main() { return f(); }
