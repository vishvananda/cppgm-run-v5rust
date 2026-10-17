// N3485 7.1.5 and 12.2p5: constexpr values survive runtime construction of their storage.
// expect exit 0
#pragma pack(push, 1)
struct Bits {
    unsigned first : 5;
    unsigned second : 30;
    constexpr Bits(unsigned first, unsigned second) : first(first), second(second) {}
};
#pragma pack(pop)
constexpr const Bits& reference = Bits(3, 7);
struct Holder { const Bits& bits; };
constexpr Holder holder = { Bits(5, 9) };
static_assert(reference.first == 3 && reference.second == 7, "semantic reference value");
static_assert(holder.bits.first == 5 && holder.bits.second == 9, "semantic member backing");
int main() {
    return reference.first == 3 && reference.second == 7 && holder.bits.first == 5 && holder.bits.second == 9 ? 0 : 1;
}
