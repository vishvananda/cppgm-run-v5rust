// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
int pick(T, int)
{
  return 2;
}

template<class T>
int pick(T *, int)
{
  return 1;
}

extern template int pick<int>(int *, int);

int use()
{
  return pick(3, 0);
}

int main()
{
  return 0;
}
