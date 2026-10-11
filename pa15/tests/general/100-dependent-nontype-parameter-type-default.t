template<class T, typename T::type Value = 0>
struct dependent_nontype_parameter_type_default
{
};

template<class T>
struct dependent_nontype_typifier
{
  typedef T type;
};

int main()
{
  dependent_nontype_parameter_type_default<
      dependent_nontype_typifier<int> > value;
  (void)&value;
  return 0;
}
