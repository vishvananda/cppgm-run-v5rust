// AUDIT-ID: ATTR-NORETURN
// AUDIT-EXPECT: reject
// N3485 7.6.3p1: a member alias-declaration declares no function.
struct table
{
  using handler [[noreturn]] = void();
};
