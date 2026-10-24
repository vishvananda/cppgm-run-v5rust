typedef _Complex double D;
typedef _Complex int I;
constexpr D a={2.0,1.0};
constexpr D b=__builtin_complex(3.0,4.0);
constexpr D c=a*b+2.0;
constexpr I q={8,3};
constexpr I r={2,1};
constexpr I s=q/r;
static_assert(__real__ c==4 && __imag__ c==11,"parts");
static_assert(a==D{2,1} && b!=a && bool(a) && !D{},"truth");
static_assert(__real__ s==4 && __imag__ s==1,"integral division");
constexpr D update() { D v=a; __imag__ v=2; v+=b; ++v; v*=2; return ~v; }
static_assert(__real__ update()==12 && __imag__ update()==-12,"mutation");
int main(){D local=c;return __real__ local==4 && __imag__ local==11 ? 0:1;}
