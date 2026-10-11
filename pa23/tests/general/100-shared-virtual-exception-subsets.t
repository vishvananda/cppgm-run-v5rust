// VALIDATION: compile-pass
// N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.
// Declaration restrictions only; no throwing function body or EH control.

namespace control_0
{
struct A{virtual void f();};struct B:virtual A{void f() throw(int);};struct C:virtual A{void f() throw(double);};struct D:B,C{void f() throw();};
}

namespace control_1
{
struct E{};struct L:virtual E{};struct R:virtual E{};struct B{virtual void f() throw(E);};struct D:B,L,R{void f() throw(D);};
}

int main() { return 0; }
