int main()
{
  char *p = new char[3];
  p[0] = 7;
  ::delete[] p;
  return 0;
}
// VALIDATION: compile-pass
