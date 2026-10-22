// N3485 5.19p2, 8.5.1: semantic constant reads reach deep subobjects.
// Constant-data representability is independent; startup must still run.
// expect exit 0
template<int N> struct Chain { Chain<N - 1> child; int value = N; };
template<> struct Chain<0> { int value = 0; };
constexpr Chain<300> deep{};
static_assert(deep.value == 300 && deep.child.child.child.value == 297, "");
int read(const int* value) { return *value; }
int main()
{
    return read(&deep.value) == 300
        && read(&deep.child.child.child.value) == 297 ? 0 : 1;
}
