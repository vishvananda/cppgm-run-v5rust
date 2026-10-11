struct S {
  char first;
  char second;
};

char S::* select_second()
{
  return &S::second;
}
