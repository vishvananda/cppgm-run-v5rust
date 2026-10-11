// VALIDATION: compile-pass
// A function-local static reference to persistent storage is initialized
// without a first-use dynamic guard.

int source = 5;

int read()
{
  static const int &reference = source;
  return reference;
}

int main()
{
  return read() == 5 ? 0 : 1;
}
