namespace casts {
typedef int V __attribute__((vector_size(8)));
typedef float F __attribute__((vector_size(8))); constexpr V v=(V)0x400000003f800000UL; constexpr F f=(F)v; constexpr unsigned long n=(unsigned long)f; static_assert(f[0]==1 && f[1]==2 && n==0x400000003f800000UL,"cast"); F g=(F)v;
}

namespace wide {
typedef unsigned long V __attribute__((vector_size(16))); constexpr unsigned __int128 n=((unsigned __int128)0x8877665544332211UL<<64)|0x123456789abcdef0UL; constexpr V v=(V)n; constexpr unsigned __int128 m=(unsigned __int128)v; static_assert(v[0]==0x123456789abcdef0UL && v[1]==0x8877665544332211UL && n==m,"wide"); V g=v;
}

namespace cast256 {
typedef unsigned char V __attribute__((vector_size(256))); typedef unsigned short W __attribute__((vector_size(256))); constexpr V v{1,2,3}; constexpr W w=(W)v; static_assert(w[0]==513 && w[1]==3 && w[127]==0,"bound");
}

namespace raw_nan {
typedef unsigned int I __attribute__((vector_size(8))); typedef float F __attribute__((vector_size(8))); constexpr I a{0x7fc00001,0xff800000}; constexpr F b=(F)a; constexpr I c=(I)b; static_assert(c[0]==0x7fc00001 && c[1]==0xff800000,"nan");
}
