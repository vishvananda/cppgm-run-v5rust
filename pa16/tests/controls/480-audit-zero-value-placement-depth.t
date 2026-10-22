// N3485 5.19, 8.5.1: active zero-initialized subobjects retain full values.
// Static image limits apply to zero values as well as nonzero values.
// expect exit 0
template<int N> struct Chain { Chain<N - 1> child; };
template<> struct Chain<0> { int value; };
union Deep { Chain<300> active; int other; };
constexpr Deep value{};
static_assert(sizeof(value) == sizeof(int), "");
int read(const int* p) { return *p; }
int main()
{
    return read(reinterpret_cast<const int*>(&value)) == 0 ? 0 : 1;
}
