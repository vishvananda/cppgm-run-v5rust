// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param], 14.3.2 [temp.arg.nontype]

template<class T>
struct limits
{
  static const unsigned digits = sizeof(T) * 8;
};

template<class T, unsigned Width,
         bool = Width < static_cast<unsigned>(limits<T>::digits)>
struct shift
{
  static const bool value = true;
};

int main()
{
  return shift<unsigned, 4>::value ? 0 : 1;
}
