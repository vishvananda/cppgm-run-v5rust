template<class T, T V>
struct integral_constant
{
  static const T value = V;
};

template<int I, class Props>
struct supportable_properties;

struct prop_a
{
};

struct prop_b
{
};

struct executor
{
};

template<int I, class Prop>
struct supportable_properties<I, void(Prop)>
{
  template<class T>
  struct is_valid_target : integral_constant<bool, true>
  {
  };
};

template<int I, class Head, class... Tail>
struct supportable_properties<I, void(Head, Tail...)>
{
  template<class T>
  struct is_valid_target :
      integral_constant<bool,
          supportable_properties<I, void(Head)>::template is_valid_target<T>::value &&
          supportable_properties<I + 1, void(Tail...)>::template is_valid_target<T>::value>
  {
  };
};

typedef void properties(prop_a, prop_b);

int main()
{
  return supportable_properties<0, properties>::is_valid_target<executor>::value ? 0 : 1;
}
