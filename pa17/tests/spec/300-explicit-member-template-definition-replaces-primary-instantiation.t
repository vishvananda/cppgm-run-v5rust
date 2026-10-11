// VALIDATION: compile-pass
// N3485 focus: 14.7.3 [temp.expl.spec]
// Clang and GCC use the explicit member-template definition for Owner<int>,
// even when the enclosing class was instantiated first.
template<class T> struct Owner
{
  template<class U> int member(U) { return 1; }
};

Owner<int> instantiated;

template<> template<class U> int Owner<int>::member(U) { return 2; }

int main()
{
  Owner<int> owner;
  return owner.member(3) == 2 ? 0 : 1;
}
