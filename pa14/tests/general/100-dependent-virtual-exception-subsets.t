// VALIDATION: compile-pass
// N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.
// Declaration restrictions only; no throwing function body or EH control.

namespace control_0
{
template<class T> struct B{virtual void f() noexcept(sizeof(T)>0);};template<class T> struct D:B<T>{void f() noexcept(sizeof(T)>0);};D<int>* p;int size() { return sizeof(D<int>); }
}

int main() { return 0; }
