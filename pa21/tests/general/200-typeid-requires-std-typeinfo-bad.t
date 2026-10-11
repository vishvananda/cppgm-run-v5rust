// VALIDATION: compile-fail

int main()
{
  return &typeid(int) ? 0 : 1;
}
