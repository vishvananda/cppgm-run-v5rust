struct Empty {};
union Value { Empty empty; int number; };
constexpr Value value{};
