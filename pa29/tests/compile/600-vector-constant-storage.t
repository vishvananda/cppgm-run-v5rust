typedef int V __attribute__((vector_size(8)));
typedef float F __attribute__((vector_size(8)));
typedef unsigned long U __attribute__((vector_size(16)));
using B = bool __attribute__((ext_vector_type(8)));
constexpr V seed{3,7};
constexpr V zero{};
constexpr V positive=+seed;
static_assert(positive[1]==7,"unary plus");
constexpr V partial{9};
constexpr V make(int n) { V v{n,n+1}; v[0]++; return v; }
constexpr int use(V v) { v[1]++; return v[0]+v[1]; }
constexpr V made=make(5);
static_assert(seed[1]==7 && zero[0]==0 && partial[1]==0,"storage");
static_assert(made[0]==6 && made[1]==6 && use(made)==13 && made[1]==6,"calls");
struct S { V v; };
constexpr int objects() {
  S s{{1,2}};
  V a[2]={{3,4},{5,6}};
  V b=a[0];
  b[0]=7;
  s.v=b;
  s.v[1]++;
  return s.v[0]+s.v[1]+a[1][0]+a[0][0];
}
static_assert(objects()==20,"objects");
constexpr int alias() { V a{1,2}; return ((V)a)[(a[0]=9,0)]; }
constexpr int list_order() { int n=0; V v{++n,++n}; return v[0]*10+v[1]; }
static_assert(alias()==9 && list_order()==12,"order");
constexpr const V& bound=V{8,9};
static_assert(bound[1]==9,"reference temporary");
constexpr F floats=(F)(V)0x400000003f800000UL;
constexpr unsigned long bytes=(unsigned long)floats;
static_assert(floats[0]==1 && floats[1]==2 && bytes==0x400000003f800000UL,"bytes");
constexpr unsigned __int128 wide=((unsigned __int128)0x8877665544332211UL<<64)|0x123456789abcdef0UL;
constexpr U lanes=(U)wide;
static_assert((unsigned __int128)lanes==wide && lanes[1]==0x8877665544332211UL,"wide bytes");
constexpr B bits{true};
constexpr B copied=bits;
struct A { constexpr operator V() const { return V{4,5}; } };
constexpr V converted=A();
constexpr const V& converted_reference=A();
static_assert(converted[1]==5 && converted_reference[0]==4,"user conversion");
V image=make(11);
V short_image{2};
F float_image{3.5f};
B packed_image{true};
V dynamic_value();
V dynamic_image=dynamic_value();
V default_image;
int access() { return image[0]+bound[0]+converted_reference[1]; }
template<int> struct LaneConstant {};
LaneConstant<V{3,4}[1]> template_lane;
template<int N=V{3,4}[0]> struct DefaultLane { static constexpr int value=N; };
static_assert(DefaultLane<>::value==3,"typed vector template argument");
