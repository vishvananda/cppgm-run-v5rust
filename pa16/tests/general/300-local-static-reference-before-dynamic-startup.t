int source = 7;
int read_constant_reference();
int observed = read_constant_reference();

int read_constant_reference()
{
  static const int &reference = source;
  return reference;
}

int selections = 0;
int other = 13;
int *choose() { ++selections; return &other; }

int &read_dynamic_reference()
{
  static int &reference = *choose();
  return reference;
}

int main()
{
  if (observed != 7 || selections != 0) return 1;
  read_dynamic_reference() = 17;
  read_dynamic_reference() = 19;
  return selections == 1 && other == 19 ? 0 : 2;
}
