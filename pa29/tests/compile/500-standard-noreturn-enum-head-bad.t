// AUDIT-ID: ATTR-NORETURN
// AUDIT-EXPECT: reject
// N3485 7.6.3p1, 7.2p1: an attribute in an enum-head appertains to the
// enumeration, which is not a function.
enum class [[noreturn]] outcome
{
  stop
};
