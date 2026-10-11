// VALIDATION: compile-pass
// N3485 focus: 14.7.1 [temp.inst], 14.7.2 [temp.explicit]

template<typename T>
T plus_one(T x)
{
  return x + 1;
}

extern template int plus_one<int>(int);

int use()
{
  return plus_one(3);
}

int main()
{
  return 0;
}
