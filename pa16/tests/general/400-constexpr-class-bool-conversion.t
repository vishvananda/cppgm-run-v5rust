struct bool_box {
  constexpr operator bool() const noexcept { return true; }
};

template<bool B>
struct sink {};

template<typename T, T v>
struct integral_constant {
  static constexpr T value = v;
  using value_type = T;

  constexpr operator value_type() const noexcept { return value; }
};

using true_type = integral_constant<bool, true>;

template<class...>
struct __and_ : true_type {};

constexpr bool box_value = bool_box{};
constexpr bool and_value = __and_<int>{};

sink<bool_box{}> box_sink;
sink<__and_<int>{}> and_sink;

static_assert(box_value, "box conversion should constant-evaluate");
static_assert(and_value, "trait conversion should constant-evaluate");
static_assert(true == __and_<int>{},
              "built-in comparison should apply the constexpr conversion");
