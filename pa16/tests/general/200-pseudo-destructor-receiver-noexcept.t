// N3485 [expr.pseudo]/1 evaluates the postfix-expression before dot/arrow.
// [expr.unary.noexcept]/3 therefore includes its potentially throwing calls.
typedef int I;
I* pointer();
I& reference();
I* safe_pointer() noexcept;
I& safe_reference() noexcept;
I* (*indirect)();
void effect();
I object;
static_assert(!noexcept(pointer()->~I()), "arrow receiver may throw");
static_assert(!noexcept(reference().~I()), "dot receiver may throw");
static_assert(!noexcept(indirect()->~I()), "indirect receiver may throw");
static_assert(!noexcept((&reference())->~I()), "address operand may throw");
static_assert(!noexcept((effect(), object).~I()), "comma operand may throw");
static_assert(!noexcept((true ? object : reference()).~I()), "potentially evaluated call");
static_assert(noexcept(safe_pointer()->~I()), "nonthrowing arrow receiver");
static_assert(noexcept(safe_reference().~I()), "nonthrowing dot receiver");
int main() { return 0; }
