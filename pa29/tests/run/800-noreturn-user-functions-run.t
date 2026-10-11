// lowir.md return=noreturn: a function declaration, definition or call
// signature states return=noreturn when the function is declared [[noreturn]]
// or GNU noreturn on any of its declarations (7.6.3): a free function, a member
// function, a function template's specializations, a virtual function's call;
// not a function template's explicit specialization without the attribute.
// Behavior only here: PA29's lanes compare no LowIR; PA32's -O0 driver lane
// compares the fact (pa32/tests/driver/o0/400-noreturn-user-functions).
