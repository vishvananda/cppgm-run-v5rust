// VALIDATION: compile-fail
// N3485 focus: 14.8.2.5 [temp.deduct.type], 8.5.4 [dcl.init.list]
// Expected: an ordinary template parameter must not deduce from a braced-init-list.

template<typename T>
void probe(T)
{
}

int main()
{
  probe({1, 2, 3});
  return 0;
}
