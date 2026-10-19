// expect reject
// N3485 8.5.4p7: a nonconstant floating value cannot narrow in a list.
int main()
{
  double value = 1.0;
  float narrowed{value};
  return (int)narrowed;
}
