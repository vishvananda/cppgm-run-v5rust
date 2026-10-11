template<int S1, int S2>
constexpr int prefix_probe(char const (&)[S1], char const (&)[S2])
{
  return S2 - 1;
}

template<class T>
int pretty_function_bound()
{
  return sizeof(char[1 + prefix_probe(__PRETTY_FUNCTION__,
                                      "int pretty_function_bound() [T = ")]) - 1;
}

int use_pretty_function_bound()
{
  return pretty_function_bound<int>();
}
