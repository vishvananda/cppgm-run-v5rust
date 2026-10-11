template<class T, T v>
struct integral_constant {
  static const T value = v;
};

typedef integral_constant<bool, true> true_type;
typedef integral_constant<bool, false> false_type;

template<class... Cond>
struct all_true : false_type {
};

template<class... T>
struct all_true<integral_constant<T, true>...> : true_type {
};

static_assert(all_true<>::value, "empty pack matches");
static_assert(all_true<integral_constant<bool, true> >::value, "one element matches");
static_assert(all_true<
                  integral_constant<bool, true>,
                  integral_constant<bool, true> >::value,
              "two elements match");
static_assert(!all_true<integral_constant<bool, false> >::value,
              "false element does not match");

int main()
{
  return all_true<
             integral_constant<bool, true>,
             integral_constant<bool, true> >::value
             ? 0
             : 1;
}
