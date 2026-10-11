// VALIDATION: compile-fail
// N3485 focus: 14.8.2.5 [temp.deduct.type]
// Expected: a function parameter pack not at the end is a non-deduced context.

template<typename T1, typename... Types>
int bad_pack(Types..., T1)
{
  return 0;
}

int main()
{
  return bad_pack(1, 2, 3);
}
