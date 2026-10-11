template <class... Ts>
struct all_integral_right
{
  static_assert((__is_integral(Ts) && ...), "");
};

template <class... Ts>
struct all_integral_left
{
  static_assert((... && __is_integral(Ts)), "");
};

template <class... Ts>
struct all_integral_init
{
  static_assert((true && ... && __is_integral(Ts)), "");
};

all_integral_right<int, long> right_ok;
all_integral_right<> right_empty_ok;
all_integral_left<int, long> left_ok;
all_integral_left<> left_empty_ok;
all_integral_init<int, long> init_ok;
all_integral_init<> init_empty_ok;

int main()
{
  return 0;
}
