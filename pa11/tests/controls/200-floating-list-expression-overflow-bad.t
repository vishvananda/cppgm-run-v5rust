// expect reject
// N3485 8.5.4p7: the constant overflows after rounding to binary32.
int main()
{
  float narrowed{1.0e38 * 4.0};
  return (int)narrowed;
}
