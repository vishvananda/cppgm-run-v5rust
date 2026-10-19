// expect exit 0
// Expression and synthesized base copies demand bodies before vtable publication.
struct Base { virtual ~Base() {} };
struct Derived : Base {};
Derived copy(Derived& source) { return source; }
int main() {
  Derived source;
  Derived result = copy(source);
  return 0;
}
