// expect reject
// The frozen walk sees the fixed surrogate first in this declaration order;
// the zero-fixed-parameter surrogate ties it instead of replacing it.
typedef int (*Variadic)(...);
typedef int (*Fixed)(int);
int variadic(...);
int fixed(int);
struct Source {
  operator Variadic() { return variadic; }
  operator Fixed() { return fixed; }
};
int main() { Source source; return source(1L); }
