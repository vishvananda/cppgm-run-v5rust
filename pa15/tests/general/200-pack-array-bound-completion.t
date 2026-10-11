// Empty packs remain valid when a known bound or another element exists.
template<int... I> int check()
{
  int inferred[] = { I..., 9 };
  int fixed[3] = { I... };
  return sizeof(inferred) / sizeof(int) == sizeof...(I) + 1 &&
         inferred[sizeof...(I)] == 9 && fixed[2] == 0 ? 0 : 1;
}
int main() { return check<>() + check<1, 2>(); }
