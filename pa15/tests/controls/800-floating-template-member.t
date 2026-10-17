// expect exit 0
// N3485 9.4.2 and 14.3.2: completed constexpr member values feed integral arguments.
template<int N> struct Source { static constexpr double value = N + 0.5; };
template<class T> struct Typed { static constexpr T value = 1.5; };
template<int N> struct Value { static const int value = N; };
static_assert(Value<int(Source<3>::value)>::value == 3, "dependent initializer");
static_assert(Value<int(Typed<double>::value)>::value == 1, "dependent type");
int main() { return int(Source<4>::value) - 4 + int(Typed<float>::value) - 1; }
