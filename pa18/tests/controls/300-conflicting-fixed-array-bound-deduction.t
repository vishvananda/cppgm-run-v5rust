// N3485 14.8.2.1: repeated fixed-element array bounds must agree;
// expect reject
template<int N> int together(char (&)[N], char (&)[N]) { return N; }
int main()
{
  char a[4], b[5];
  return together(a, b);
}
