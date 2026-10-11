// VALIDATION: compile-fail
// N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.
// Declaration restrictions only; no throwing function body or EH control.

namespace control_0
{
struct A{virtual void f();};struct B:virtual A{void f() throw(int);};struct C:virtual A{void f() throw(double);};struct D:B,C{void f() throw(int,double);};
}

int main() { return 0; }
