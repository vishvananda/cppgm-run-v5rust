// VALIDATION: emit-lowir
// N3485 focus: 14.7.3 [temp.expl.spec], 7.1.2 [dcl.fct.spec]

template<class T>
inline int digits(T)
{
  return 1;
}

template<>
inline int digits(unsigned int x)
{
  return x == 4U ? 4 : 0;
}

template<>
inline int digits(unsigned long long x)
{
  return x == 9ULL ? 9 : 0;
}

int main()
{
  return digits(4U) + digits(9ULL) - 13;
}
