// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
int plus_one(T x)
{
  return x + 1;
}

extern template int plus_one(int);

int main()
{
  return plus_one(0);
}
