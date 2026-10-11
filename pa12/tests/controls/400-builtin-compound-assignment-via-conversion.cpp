// N3485 13.6p18, 13.3.1.2p7: for s += 2 with S::operator int&(), the built-in
// candidate int& operator+=(int&, int) applies after the conversion (++s
// already works); expect exit 0
int g = 4;
struct S { operator int&() { return g; } };
int main() { S s; s += 2; s *= 3; return g == 18 ? 0 : 1; }
