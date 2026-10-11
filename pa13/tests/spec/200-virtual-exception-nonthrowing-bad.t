// VALIDATION: compile-fail
// N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.
// Declaration restrictions only; no throwing function body or EH control.

namespace control_0
{
struct B{virtual void f() noexcept;};struct D:B{void f() noexcept(false) override;};
}

int main() { return 0; }
