int f()
{
  static const char * values[] = {"a", "bb"};
  static const char nested[1][2] = {"x"};
  return values[0][0] + values[1][1] + nested[0][1];
}

int main()
{
  return f() == ('a' + 'b') ? 0 : 1;
}
