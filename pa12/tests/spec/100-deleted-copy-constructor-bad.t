// VALIDATION: compile-fail
// N3485 focus: 8.4.3 [dcl.fct.def.delete], 12.8 [class.copy]
// Expected: deleted copy constructor blocks copy-initialization.

struct CopyBlocked
{
  CopyBlocked() {}
  CopyBlocked(const CopyBlocked &) = delete;
};

int main()
{
  CopyBlocked x;
  CopyBlocked y = x;
  return 0;
}
