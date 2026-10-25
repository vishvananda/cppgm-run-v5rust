typedef int V __attribute__((vector_size(8)));
constexpr const V& f(V v){return v;} constexpr int n=f(V{1,2})[0];
