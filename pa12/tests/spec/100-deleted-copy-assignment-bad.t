// VALIDATION: compile-fail
// N3485 focus: 8.4.3 [dcl.fct.def.delete], 12.8 [class.copy]
// Expected: deleted copy assignment blocks lvalue assignment.

struct MoveOnly
{
  MoveOnly() {}
  MoveOnly(const MoveOnly &) = delete;
  MoveOnly(MoveOnly &&) = default;
  MoveOnly & operator=(const MoveOnly &) = delete;
  MoveOnly & operator=(MoveOnly &&) = default;
};

int main()
{
  MoveOnly a;
  MoveOnly b;
  a = b;
  return 0;
}
