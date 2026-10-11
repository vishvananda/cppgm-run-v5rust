// VALIDATION: compile-pass
// N3485 focus: 14.7.1 [temp.inst]

template<typename T>
int unused_body()
{
  return sizeof(typename T::missing_type);
}

int main()
{
  return 0;
}
