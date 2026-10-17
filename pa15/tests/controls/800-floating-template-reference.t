// expect exit 0
// N3485 5.19: a constant binding and a cast temporary retain typed scalar storage.
constexpr double value = 2.5;
constexpr const double &reference = value;
constexpr const double &temporary = 3;
template<int N> struct Value { static const int value = N; };
static_assert(Value<int(reference)>::value == 2, "bound object");
static_assert(Value<int(temporary)>::value == 3, "converted reference temporary");
static_assert(Value<int(static_cast<const float&>(1.5))>::value == 1, "cast temporary");
static_assert(Value<int(static_cast<const double&>(2))>::value == 2, "converted cast temporary");
int main() { return int(reference) + int(temporary) - 5; }
