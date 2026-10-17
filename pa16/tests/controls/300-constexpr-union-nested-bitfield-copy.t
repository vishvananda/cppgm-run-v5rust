// N3485 5.19p2, 9.5p1, 9.6: a bit-field write in a union's active class
// member preserves that variant when the class or union is copied; expect exit 0
struct Bits { unsigned n : 3; };
union U { Bits a; Bits b; };
constexpr Bits local() { U u = {{3}}; u.a.n = 5; return u.a; }
constexpr U published() { U u = {{2}}; u.a.n = 7; return u; }
constexpr Bits direct = local();
constexpr U u = published();
constexpr Bits copied = u.a;
static_assert(direct.n == 5 && copied.n == 7, "active class bit-field");
int main() { return direct.n == 5 && u.a.n == 7 ? 0 : 1; }
