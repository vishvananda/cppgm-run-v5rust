// VALIDATION: compile-pass
// N3485 8.3.5p9: a by-value parameter's class type is complete in a function
// definition even when no expression uses the parameter. Spelled through an
// alias template, the parameter names a specialization nothing else
// instantiates, so the definition itself must complete it.
template<class T> struct Box { T value; ~Box() {} };
template<class T> using Boxed = Box<T>;
template<class T> struct Plain { T value; };
template<class T> using Bare = Plain<T>;
int unnamed(Boxed<int>) { return 1; }
int named(Boxed<char> box) { return 2; }
int trivial(Bare<long>) { return 3; }
template<class T> int deduced(Boxed<T>) { return 4; }
template int deduced(Boxed<short>);
int main() { return 0; }
