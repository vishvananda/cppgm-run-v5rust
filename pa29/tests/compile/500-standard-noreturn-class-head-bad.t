// AUDIT-ID: ATTR-NORETURN
// AUDIT-EXPECT: reject
// N3485 7.6.3p1, 9p1: an attribute in a class-head appertains to the class,
// which is not a function.
struct [[noreturn]] handler
{
  void run();
};
