// N3485 4.7, 4.12, 5.7, 5.8, 5.9; supported __int128 extension.
// expect exit 0
using Signed = __int128_t;
using Unsigned = __uint128_t;
Unsigned shift(Unsigned value, int count) { return value << count; }
Signed widen(int value) { return value; }
bool below(Unsigned a, Unsigned b) { return a < b; }
bool truth(Unsigned value) { return !!value; }
int main()
{
    Unsigned high = shift(1, 100);
    Unsigned low = 9;
    Signed negative = widen(-1);
    if (sizeof(high) != 16 || alignof(Unsigned) != 16) return 1;
    if (!truth(high) || truth(0) || !below(low, high)) return 2;
    if (negative < low || !(low < negative)) return 3;
    if ((high >> 100) != 1 || (unsigned long long)(high + low) != 9) return 4;
    if ((high * 3 / 3) != high || (high * 3 % 3) != 0) return 5;
    if ((Unsigned)negative != ~Unsigned(0)) return 6;
    return 0;
}
