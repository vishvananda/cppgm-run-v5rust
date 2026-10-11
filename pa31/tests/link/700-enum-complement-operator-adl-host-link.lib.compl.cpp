namespace flags {
namespace detail {
enum mode { none = 0, one = 1, two = 2, four = 4 };
mode operator|(mode x, mode y) { return mode(int(x) | int(y)); }
mode operator&(mode x, mode y) { return mode(int(x) & int(y)); }
mode operator~(mode x) { return mode(~int(x) & 0xF); }
}
}
