// VALIDATION: compile-fail
// N3485 focus: 14.7.3 [temp.expl.spec]
// Clang rejects a second explicit definition of the same member template.
template<class T> struct Owner
{
  template<class U> void member(U);
};

template<> template<class U> void Owner<int>::member(U) {}
template<> template<class U> void Owner<int>::member(U) {}

int main() { return 0; }
