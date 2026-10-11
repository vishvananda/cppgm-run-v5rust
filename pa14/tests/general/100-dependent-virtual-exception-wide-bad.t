// VALIDATION: compile-fail
// N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.
// Declaration restrictions only; no throwing function body or EH control.

namespace control_0
{
template<class T> struct B{virtual void f() throw(T);};template<class T> struct D:B<T>{void f() throw(double);};int size() { return sizeof(D<int>); }
}

int main() { return 0; }
