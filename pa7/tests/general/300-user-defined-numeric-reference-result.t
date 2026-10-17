// N3485 2.14.8p1, 5.2.2p10: a literal call keeps its referent expression type.
int value;
int &operator ""_ref(unsigned long long) { return value; }
int main()
{
  0_ref = 4;
  3_ref;
  return 4_ref;
}
