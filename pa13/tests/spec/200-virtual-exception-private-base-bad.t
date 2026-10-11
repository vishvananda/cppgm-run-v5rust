// VALIDATION: compile-fail
// N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.
// Declaration restrictions only; no throwing function body or EH control.

namespace control_0
{
struct E{};class F:E{};struct B{virtual void f() throw(E);};struct D:B{void f() throw(F) override;};
}

int main() { return 0; }
