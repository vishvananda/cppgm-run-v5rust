// expect exit 0
// The frozen surrogate walk keeps declaration order and default standard
// sequences for variadic argument slots; it compares fixed parameters only.
typedef int (*Variadic)(...);
typedef int (*Fixed)(int);
int variadic(...) { return 7; }
int fixed(int) { return 8; }
struct Source {
  operator Fixed() { return fixed; }
  operator Variadic() { return variadic; }
};
int main() { Source source; return source(1L) != 7; }
