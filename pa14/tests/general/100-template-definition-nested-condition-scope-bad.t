template<class T>
int invalid_nested_condition_scope()
{
  if (true) {
    return later;
    if (int later = 0) {}
  }
  return 0;
}
