// PA29 GNU typeof expressions check temporary destruction, unlike the final
// call of an ordinary decltype operand.
struct S { ~S() = delete; };
S make();
typedef __typeof__(make()) result;
